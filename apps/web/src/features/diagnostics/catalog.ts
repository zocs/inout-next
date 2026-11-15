import { interpolate, type Locale } from "@/i18n/catalog";
// This catalog is imported only by the lazy diagnostics panel.
const messages = [
 ["The local action failed. Try again.", "本地操作失败。请重试。", "本機操作失敗。請重試。"],
 ["Buffer applied to subsequent requests.", "缓冲区已应用于后续请求。", "緩衝區已套用至後續請求。"], ["Default buffer restored.", "已恢复默认缓冲区。", "已還原預設緩衝區。"], ["Local test canceled", "已取消本地测试", "已取消本機測試"],
 ["Local diagnostics", "本地诊断", "本機診斷"], ["Closing…", "正在关闭…", "正在關閉…"], ["Close diagnostics", "关闭诊断", "關閉診斷"],
 ["Temporary settings for this sharing session. Buffer changes apply to subsequent downloads; closing diagnostics or stopping sharing restores the default.", "此共享会话的临时设置。缓冲区更改应用于后续下载；关闭诊断或停止共享会恢复默认值。", "此共享工作階段的暫時設定。緩衝區變更套用至後續下載；關閉診斷或停止共享會還原預設值。"],
 ["Start sharing to use local diagnostics.", "开始共享后可使用本地诊断。", "開始共享後可使用本機診斷。"], ["Connecting to local diagnostics…", "正在连接本地诊断…", "正在連線至本機診斷…"], ["Running a read-only local test…", "正在运行只读本地测试…", "正在執行唯讀本機測試…"],
 ["Download buffer", "下载缓冲区", "下載緩衝區"], ["Test connections", "测试连接数", "測試連線數"], ["Relative shared-file path", "共享文件相对路径", "共享檔案相對路徑"], ["Current buffer: {0} KiB", "当前缓冲区：{0} KiB", "目前緩衝區：{0} KiB"],
 ["Working…", "正在处理…", "正在處理…"], ["Apply buffer", "应用缓冲区", "套用緩衝區"], ["Restore default buffer", "恢复默认缓冲区", "還原預設緩衝區"], ["Run read-only test", "运行只读测试", "執行唯讀測試"], ["Cancel test", "取消测试", "取消測試"],
 ["Reads up to the first 64 MiB through this device’s shared-file service without saving data. The test ends within 30 seconds. This same-device timing sample measures local throughput; it does not prove file integrity or external network speed.", "通过此设备的共享文件服务读取前 64 MiB，不保存数据。测试在 30 秒内结束。此同设备计时样本测量本地吞吐量，不能证明文件完整性或外部网络速度。", "透過此裝置的共享檔案服務讀取前 64 MiB，不儲存資料。測試在 30 秒內結束。此同裝置計時樣本測量本機吞吐量，無法證明檔案完整性或外部網路速度。"],
 ["Local throughput sample", "本地吞吐量样本", "本機吞吐量樣本"], ["Local throughput", "本地吞吐量", "本機吞吐量"], ["Bytes read", "读取字节数", "讀取位元組數"], ["Duration", "耗时", "耗時"], ["Sample settings", "样本设置", "樣本設定"], ["{0} KiB buffer with {1} connections", "{0} KiB 缓冲区，{1} 个连接", "{0} KiB 緩衝區，{1} 個連線"],
] as const;
export type DiagnosticKey = typeof messages[number][0];
export type DiagnosticMessage = string | Error | { key: DiagnosticKey };
const catalogs = Object.fromEntries(["en", "zh-Hans", "zh-Hant"].map((locale, index) => [locale, Object.fromEntries(messages.map(row => [row[0], row[index]]))])) as Record<Locale, Record<DiagnosticKey, string>>;
export function diagnosticText(locale: Locale, key: DiagnosticKey, ...args: (string | number)[]) { return interpolate(catalogs[locale][key], args); }
export function diagnosticMessage(locale: Locale, message: DiagnosticMessage): string {
  const text = message instanceof Error ? message.message : typeof message === "string" ? message : message.key;
  return Object.hasOwn(catalogs[locale], text) ? catalogs[locale][text as DiagnosticKey] : text;
}
