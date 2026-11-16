#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    English,
    Simplified,
    Traditional,
}

impl Locale {
    pub fn from_tag(tag: &str) -> Option<Self> {
        let normalized = tag.trim().split(['.', '@']).next()?.replace('_', "-");
        let parts: Vec<_> = normalized.split('-').collect();
        let language = parts.first()?;
        if parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_alphanumeric()))
        {
            return None;
        }
        if !language.eq_ignore_ascii_case("en") && !language.eq_ignore_ascii_case("zh") {
            return None;
        }
        let mut script = None;
        let mut region = None;
        let mut extlangs = 0;
        for (index, part) in parts.iter().enumerate().skip(1) {
            if part.len() == 1 {
                if !valid_extensions(&parts[index..]) {
                    return None;
                }
                break;
            }
            if part.len() == 4 && part.bytes().all(|byte| byte.is_ascii_alphabetic()) {
                if script.is_some() || region.is_some() {
                    return None;
                }
                script = Some(*part);
            } else if (part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_alphabetic()))
                || (part.len() == 3 && part.bytes().all(|byte| byte.is_ascii_digit()))
            {
                if region.is_some() {
                    return None;
                }
                region = Some(*part);
            } else if part.len() == 3
                && part.bytes().all(|byte| byte.is_ascii_alphabetic())
                && script.is_none()
                && region.is_none()
                && extlangs < 3
            {
                extlangs += 1;
            } else if !((5..=8).contains(&part.len())
                || (part.len() == 4 && part.as_bytes()[0].is_ascii_digit()))
            {
                return None;
            }
        }
        if language.eq_ignore_ascii_case("en") {
            return Some(Self::English);
        }
        if script.is_some_and(|value| value.eq_ignore_ascii_case("Hans")) {
            return Some(Self::Simplified);
        }
        let traditional = script.is_some_and(|value| value.eq_ignore_ascii_case("Hant"))
            || region.is_some_and(|value| {
                ["TW", "HK", "MO"]
                    .iter()
                    .any(|region| value.eq_ignore_ascii_case(region))
            });
        Some(if traditional {
            Self::Traditional
        } else {
            Self::Simplified
        })
    }

    pub fn from_index(index: i32) -> Self {
        match index {
            1 => Self::Simplified,
            2 => Self::Traditional,
            _ => Self::English,
        }
    }

    pub fn index(self) -> i32 {
        match self {
            Self::English => 0,
            Self::Simplified => 1,
            Self::Traditional => 2,
        }
    }
}

fn valid_extensions(mut parts: &[&str]) -> bool {
    let mut seen = [false; 36];
    while let Some(singleton) = parts.first() {
        if singleton.len() != 1 {
            return false;
        }
        let token = singleton.as_bytes()[0].to_ascii_lowercase();
        parts = &parts[1..];
        if token == b'x' {
            return !parts.is_empty() && parts.iter().all(|part| (1..=8).contains(&part.len()));
        }
        let index = if token.is_ascii_digit() {
            usize::from(token - b'0')
        } else {
            usize::from(token - b'a') + 10
        };
        if std::mem::replace(&mut seen[index], true) {
            return false;
        }
        let count = parts.iter().take_while(|part| part.len() != 1).count();
        if count == 0
            || !parts[..count]
                .iter()
                .all(|part| (2..=8).contains(&part.len()))
        {
            return false;
        }
        parts = &parts[count..];
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidPort;

pub fn parse_port(input: &str) -> Result<u16, InvalidPort> {
    let input = input.trim();
    if input.is_empty() || !input.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(InvalidPort);
    }
    input.parse().map_err(|_| InvalidPort)
}

macro_rules! catalog {
    ($($key:ident => ($english:literal, $simplified:literal, $traditional:literal)),+ $(,)?) => {
        #[derive(Clone, Copy)]
        pub enum TextKey { $($key),+ }
        pub fn text(locale: Locale, key: TextKey) -> &'static str {
            let values = match key { $(TextKey::$key => [$english, $simplified, $traditional]),+ };
            values[locale.index() as usize]
        }
    };
}

catalog! {
    ShareFolder => ("Share a folder", "分享文件夹", "分享資料夾"),
    Intro => ("Transfer files through a browser on your network.", "同一网络中的设备可通过浏览器传输文件。", "同一網路中的裝置可透過瀏覽器傳輸檔案。"),
    Folder => ("Folder", "文件夹", "資料夾"),
    ChooseFolder => ("Choose folder", "选择文件夹", "選擇資料夾"),
    FolderHint => ("Choose the folder you want other devices to access.", "选择希望其他设备访问的文件夹。", "選擇希望其他裝置存取的資料夾。"),
    Start => ("Start sharing", "开始分享", "開始分享"),
    Stop => ("Stop sharing", "停止分享", "停止分享"),
    OpenBrowser => ("Open in browser", "在浏览器打开", "在瀏覽器開啟"),
    CopyAddress => ("Copy address", "复制地址", "複製位址"),
    Address => ("Share address", "分享地址", "分享位址"),
    NoAddress => ("Start sharing to get a share address.", "开始分享后，这里会显示访问地址。", "開始分享後，這裡會顯示存取位址。"),
    Key => ("Access key", "访问口令", "存取口令"),
    KeyHint => ("Optional. Public uploads and management are allowed; deletion requires a key.", "口令可选。可公开上传和管理文件；删除文件必须设置口令。", "口令可選。可公開上傳和管理檔案；刪除檔案必須設定口令。"),
    Upload => ("Allow uploads", "允许上传", "允許上傳"),
    Manage => ("Allow file management", "允许管理文件", "允許管理檔案"),
    Network => ("Allow other devices on this network", "允许同一网络中的其他设备访问", "允許同一網路中的其他裝置存取"),
    Port => ("Port", "端口", "連接埠"),
    PortHint => ("Use 0 to choose an available port.", "设为 0 可自动选择可用端口。", "設為 0 可自動選擇可用連接埠。"),
    Running => ("Sharing is active", "正在分享", "正在分享"),
    Stopped => ("Sharing is stopped", "尚未开始分享", "尚未開始分享"),
    Busy => ("Please wait…", "请稍候…", "請稍候…"),
    ConfirmStop => ("Stop sharing?", "确定停止分享？", "確定停止分享？"),
    ConfirmClose => ("Close the application?", "确定关闭应用？", "確定關閉應用程式？"),
    ConfirmMessage => ("Stopping will interrupt any transfers that are still running.", "停止服务将中断尚未完成的传输。", "停止服務將中斷尚未完成的傳輸。"),
    Cancel => ("Keep sharing", "继续分享", "繼續分享"),
    Confirm => ("Stop now", "确定停止", "確定停止"),
    Transfers => ("Transfers", "传输记录", "傳輸記錄"),
    TransferHint => ("Records reset when sharing starts again. Sent bytes do not confirm saving on another device.", "重新开始分享会清空记录；已发送字节不代表对方设备已保存。", "重新開始分享會清空記錄；已傳送位元組不代表對方裝置已儲存。"),
    NoTransfers => ("Transfers will appear here when another device sends or receives a file.", "其他设备上传或下载文件后，记录会显示在这里。", "其他裝置上傳或下載檔案後，記錄會顯示在這裡。"),
    UploadKind => ("Upload", "上传", "上傳"),
    DownloadKind => ("Download", "下载", "下載"),
    CopyKind => ("Copy", "复制", "複製"),
    Active => ("In progress", "传输中", "傳輸中"),
    Published => ("Published", "已提交", "已提交"),
    Served => ("Sent", "已发送", "已傳送"),
    Interrupted => ("Interrupted", "已中断", "已中斷"),
    Failed => ("Failed", "失败", "失敗"),
    About => ("About", "关于", "關於"),
    CloseAbout => ("Close about", "关闭关于", "關閉關於"),
    Preview => ("Native interface trial", "原生界面试验", "原生介面試驗"),
    Language => ("Language", "语言", "語言"),
    Theme => ("Theme", "主题", "主題"),
    Evergreen => ("Evergreen", "常青", "常青"),
    Tangerine => ("Tangerine", "柑橘", "柑橘"),
    Ocean => ("Ocean", "海蓝", "海藍"),
    Berry => ("Berry", "莓紫", "莓紫"),
    ReduceMotion => ("Reduce startup motion", "减少启动动画", "減少啟動動畫"),
    ErrorSelect => ("Could not select that folder. Choose another folder.", "无法使用该文件夹，请重新选择。", "無法使用該資料夾，請重新選擇。"),
    ErrorStart => ("Could not start sharing. Check the folder, key and port, then try again.", "无法开始分享，请检查文件夹、口令和端口后重试。", "無法開始分享，請檢查資料夾、口令及連接埠後重試。"),
    ErrorStop => ("Sharing stopped with a cleanup error. Check interrupted transfers before retrying.", "服务停止时清理未能完成，请检查中断的传输后重试。", "服務停止時清理未能完成，請檢查中斷的傳輸後重試。"),
    ErrorPort => ("Enter a whole port number from 0 to 65535.", "请输入 0 至 65535 之间的整数端口。", "請輸入 0 至 65535 之間的整數連接埠。"),
    ErrorCopy => ("Could not copy the address. Select it and copy manually.", "无法复制地址，可选中地址后手动复制。", "無法複製位址，可選取位址後手動複製。"),
    ErrorBrowser => ("Could not open a browser. Copy the address and open it manually.", "无法打开浏览器，可复制地址后手动打开。", "無法開啟瀏覽器，可複製位址後手動開啟。"),
    ErrorPicker => ("Could not open the folder chooser. Enter the folder path instead.", "无法打开文件夹选择器，可直接输入文件夹路径。", "無法開啟資料夾選擇器，可直接輸入資料夾路徑。"),
    AddressCopied => ("Address copied", "已复制地址", "已複製位址"),
    Public => ("Public access", "公开访问", "公開存取"),
    Protected => ("Access key required", "访问需要口令", "存取需要口令"),
    Opening => ("Opening…", "正在打开…", "正在開啟…"),
    LocalOnly => ("Only this device", "仅此设备", "僅此裝置"),
    ByteUnit => ("bytes", "字节", "位元組"),
    NoAccounting => ("Size unavailable", "大小未知", "大小未知"),
    ErrorCommand => ("Could not send this action. Wait a moment and try again.", "无法执行该操作，请稍候重试。", "無法執行此操作，請稍候再試。"),
    ErrorAssets => ("The web resources are missing. Reinstall the complete portable package.", "缺少网页资源，请重新安装完整的便携包。", "缺少網頁資源，請重新安裝完整的可攜套件。"),
    ReadinessError => ("Could not initialize the native host. Restart the application.", "无法初始化原生服务，请重新打开应用。", "無法初始化原生服務，請重新開啟應用程式。"),
}
