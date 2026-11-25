param(
    [Parameter(Mandatory=$true)][string]$BinaryPath,
    [Parameter(Mandatory=$true)][string]$ExpectedTitle,
    [Parameter(Mandatory=$true)][string]$ReportPath,
    [ValidateRange(1,60)][int]$TimeoutSeconds = 20,
    [switch]$VerifyChoices
)

# UTF-8 BOM preserves the native CJK choice probes in Windows PowerShell 5.
# Real default-renderer startup and normal close. No sharing is started by this
# check; interactive file/input/transfer and presented-frame checks are separate.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$Binary = (Resolve-Path -LiteralPath $BinaryPath).Path
$Report = [IO.Path]::GetFullPath($ReportPath)
$ReportDirectory = Split-Path $Report -Parent
New-Item -ItemType Directory -Path $ReportDirectory -Force | Out-Null
if (Test-Path -LiteralPath $Report) { throw 'Preserve the original startup report; use a new path' }
$Stdout = $Report + '.stdout.log'
$Stderr = $Report + '.stderr.log'
if ((Test-Path -LiteralPath $Stdout) -or (Test-Path -LiteralPath $Stderr)) { throw 'Preserve original startup logs' }
$WorkingDirectory = Join-Path ([IO.Path]::GetTempPath()) ('inout-native-startup-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $WorkingDirectory | Out-Null
$OldTrace = [Environment]::GetEnvironmentVariable('INOUT_NATIVE_TRACE','Process')
$OldBackend = [Environment]::GetEnvironmentVariable('SLINT_BACKEND','Process')
$OldPreferences = [Environment]::GetEnvironmentVariable('INOUT_NATIVE_PREFERENCES','Process')
$Preferences = Join-Path $WorkingDirectory 'native-preferences.conf'
$Native = $null
$ClosePosted = $false
$Failure = $null
$Result = [ordered]@{
    result='failed'; binary=$Binary; binaryBytes=(Get-Item -LiteralPath $Binary).Length;
    binarySha256=(Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant();
    expectedTitle=$ExpectedTitle; defaultRenderer=$true; unrelatedWorkingDirectory=$WorkingDirectory;
    isolatedPreferences=$Preferences;
    measurement='controller-ready callback before presentation'; startedAt=[DateTime]::UtcNow.ToString('o')
}

function Read-SharedLog([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return '' }
    $Stream = New-Object IO.FileStream($Path,[IO.FileMode]::Open,[IO.FileAccess]::Read,([IO.FileShare]::ReadWrite -bor [IO.FileShare]::Delete))
    $Reader = New-Object IO.StreamReader($Stream,[Text.Encoding]::UTF8)
    try { $Reader.ReadToEnd() } finally { $Reader.Dispose(); $Stream.Dispose() }
}

if (-not ('InoutNativeStartup.WindowMethods' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace InoutNativeStartup {
    public static class WindowMethods {
        [DllImport("user32.dll")]
        public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
        [DllImport("user32.dll", SetLastError=true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    }
}
'@
}

function Close-OwnedWindow {
    $Native.Refresh()
    if ($Native.HasExited) { return }
    $Window = $Native.MainWindowHandle
    if ($Window -eq [IntPtr]::Zero) { throw 'Owned process has no window to close normally' }
    [uint32]$WindowProcess = 0
    [void][InoutNativeStartup.WindowMethods]::GetWindowThreadProcessId($Window,[ref]$WindowProcess)
    if ($WindowProcess -ne $Native.Id) { throw 'Refuse to close a window belonging to another process' }
    if (-not [InoutNativeStartup.WindowMethods]::PostMessage($Window,0x0010,[IntPtr]::Zero,[IntPtr]::Zero)) {
        throw 'Normal close was not delivered to the owned native window'
    }
    $script:ClosePosted = $true
}

function Verify-OwnedChoices {
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    $LocaleNames = @('Language','语言','語言')
    $ThemeNames = @('Theme','主题','主題')
    function Find-Choice([string[]]$Names) {
        $FindDeadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            $Window = [Windows.Automation.AutomationElement]::FromHandle($Native.MainWindowHandle)
            if ($Window.Current.ProcessId -ne $Native.Id) { throw 'Choice window belongs to another process' }
            $Fields = @($Window.FindAll([Windows.Automation.TreeScope]::Descendants,(New-Object Windows.Automation.PropertyCondition([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::ComboBox))) | Where-Object {$_.Current.ProcessId -eq $Native.Id -and $_.Current.Name -in $Names})
            if ($Fields.Count -gt 1) { throw 'Owned native choice is not unique' }
            if ($Fields.Count -eq 1 -and $Fields[0].Current.IsEnabled) { return $Fields[0] }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $FindDeadline)
        throw 'Owned native choice did not become enabled'
    }
    function Wait-Choice([string[]]$Names,[string]$ExpectedLabel,[string]$ExpectedValue) {
        $Deadline = [DateTime]::UtcNow.AddSeconds(5)
        do {
            $Field = Find-Choice $Names
            $Value = $Field.GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).Current.Value
            if ($Field.Current.Name -eq $ExpectedLabel -and $Value -eq $ExpectedValue) { return }
            Start-Sleep -Milliseconds 100
        } while ([DateTime]::UtcNow -lt $Deadline)
        throw ('Native choice did not update its value and localized label: ' + $ExpectedLabel)
    }
    $InitialLocale = (Find-Choice $LocaleNames).Current.Name
    (Find-Choice $LocaleNames).GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('简体中文')
    Wait-Choice $LocaleNames '语言' '简体中文'
    (Find-Choice $LocaleNames).GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('English')
    Wait-Choice $LocaleNames 'Language' 'English'
    (Find-Choice $ThemeNames).GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('Ocean')
    Wait-Choice $ThemeNames 'Theme' 'Ocean'
    (Find-Choice $LocaleNames).GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('繁體中文')
    Wait-Choice $LocaleNames '語言' '繁體中文'
    Wait-Choice $ThemeNames '主題' '海藍'
    (Find-Choice $LocaleNames).GetCurrentPattern([Windows.Automation.ValuePattern]::Pattern).SetValue('English')
    Wait-Choice $LocaleNames 'Language' 'English'
    Wait-Choice $ThemeNames 'Theme' 'Ocean'
    return @{result='pass';initialLocaleLabel=$InitialLocale;locales=@('简体中文','English','繁體中文','English');theme='Ocean';themePreservedAcrossLocales=$true;scope='owned native UI Automation values and localized labels; no sharing or presented-frame measurement'}
}

try {
    [Environment]::SetEnvironmentVariable('INOUT_NATIVE_TRACE','1','Process')
    [Environment]::SetEnvironmentVariable('SLINT_BACKEND',$null,'Process')
    # Choice probes and ordinary close save drafts. Keep those writes inside
    # this audit fixture, including when the caller overrides its own prefs.
    [Environment]::SetEnvironmentVariable('INOUT_NATIVE_PREFERENCES',$Preferences,'Process')
    $Native = Start-Process -FilePath $Binary -WorkingDirectory $WorkingDirectory -RedirectStandardOutput $Stdout -RedirectStandardError $Stderr -PassThru
    # Retain the process handle: Windows PowerShell5 can otherwise lose its
    # exit code between an asynchronous Start-Process and WaitForExit.
    [void]$Native.Handle
    $Result.pid = $Native.Id
    $Deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
    do {
        $Native.Refresh()
        $Log = Read-SharedLog $Stderr
        if ($Native.HasExited) { throw ('Native process exited before usable window/controller readiness: ' + $Log) }
        $Ready = ($Native.MainWindowHandle -ne [IntPtr]::Zero) -and ($Native.MainWindowTitle -eq $ExpectedTitle) -and ($Log -match 'native-host-ready-callback')
        if ($Ready) { break }
        Start-Sleep -Milliseconds 100
    } while ([DateTime]::UtcNow -lt $Deadline)
    if (-not $Ready) { throw 'Native window/controller did not become ready within the bounded startup wait' }
    $Result.windowTitle = $Native.MainWindowTitle
    $Result.sessionId = $Native.SessionId
    $Result.windowHandle = $Native.MainWindowHandle.ToInt64()
    if ($Log -match 'native-host-ready-callback elapsed_ms=([0-9.]+)') {
        $Result.controllerReadyCallbackMs = [double]::Parse($Matches[1],[Globalization.CultureInfo]::InvariantCulture)
    }
    if ($VerifyChoices) { $Result.choices = Verify-OwnedChoices }
    Close-OwnedWindow
    if (-not $Native.WaitForExit(10000)) { throw 'Owned idle window did not finish normal close; process retained for investigation' }
    $Result.exitCode = $Native.ExitCode
    if ($Native.ExitCode -ne 0) { throw ('Native normal close returned nonzero exit: ' + $Native.ExitCode) }
    $Result.result = 'pass'
} catch {
    $Failure = $_.Exception.Message
    $Result.failure = $Failure
} finally {
    $Finalization = [ordered]@{}
    try {
        if (($null -ne $Native) -and (-not $ClosePosted)) {
            try {
                $Native.Refresh()
                if (-not $Native.HasExited) {
                    Close-OwnedWindow
                    [void]$Native.WaitForExit(10000)
                }
            } catch { $Result.cleanupFailure = $_.Exception.Message }
        }
        $Result.closePosted = $ClosePosted
        $Result.stdout = ''
        $Result.stderr = ''
        try { $Result.stdout = Read-SharedLog $Stdout }
        catch { $Finalization.stdout = $_.Exception.Message }
        try { $Result.stderr = Read-SharedLog $Stderr }
        catch { $Finalization.stderr = $_.Exception.Message }
        $Result.completedAt = [DateTime]::UtcNow.ToString('o')
    } finally {
        try {
            [Environment]::SetEnvironmentVariable('INOUT_NATIVE_TRACE',$OldTrace,'Process')
        } catch { $Finalization.traceRestore = $_.Exception.Message }
        finally {
            try {
                [Environment]::SetEnvironmentVariable('SLINT_BACKEND',$OldBackend,'Process')
            } catch { $Finalization.backendRestore = $_.Exception.Message }
            finally {
                try {
                    [Environment]::SetEnvironmentVariable('INOUT_NATIVE_PREFERENCES',$OldPreferences,'Process')
                } catch { $Finalization.preferencesRestore = $_.Exception.Message }
                finally {
                    if ($null -ne $Native) {
                        try { $Native.Dispose() }
                        catch { $Finalization.processDispose = $_.Exception.Message }
                    }
                }
            }
        }
    }
    if ($Finalization.Count -gt 0) {
        $Result.finalizationFailures = $Finalization
        $Result.result = 'failed'
        if ($null -eq $Failure) {
            $Failure = 'Native startup verification finalization failed'
            $Result.failure = $Failure
        }
    }
    $Json = $Result | ConvertTo-Json -Depth 8
    try {
        [IO.File]::WriteAllText($Report,$Json,(New-Object Text.UTF8Encoding($false)))
    } catch {
        $Finalization.report = $_.Exception.Message
        $Result.finalizationFailures = $Finalization
        $Result.result = 'failed'
        if ($null -eq $Failure) {
            $Failure = 'Native startup verification report could not be written'
            $Result.failure = $Failure
        }
        $Json = $Result | ConvertTo-Json -Depth 8
    }
    # Retain the empty unrelated cwd with the evidence. Windows can still hold
    # its directory handle briefly after exit; deletion must not hide a failed
    # renderer startup or normal-close result.
    Write-Output $Json
}
if ($null -ne $Failure) { throw $Failure }
