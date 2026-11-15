export const locales = ["en", "zh-Hans", "zh-Hant"] as const;
export type Locale = typeof locales[number];
// Complete phrases keep copy and word order explicit; data stays in arguments.
const messages = [
 ["Language", "语言", "語言"], ["Theme", "主题", "主題"], ["System", "跟随系统", "跟隨系統"],
 ["Evergreen", "常青", "常青"], ["Tangerine", "橘光", "橘光"], ["Ocean", "海蓝", "海藍"], ["Berry", "莓紫", "莓紫"],
 ["Opening…", "正在打开…", "正在開啟…"], ["Opening {0}", "正在打开 {0}", "正在開啟 {0}"],
 ["Cannot connect", "无法连接", "無法連線"], ["Opening shared files", "正在打开共享文件", "正在開啟共享檔案"], ["Try again", "重试", "重試"],
 ["Places", "位置", "位置"], ["Go to shared folder", "转到共享文件夹", "前往共享資料夾"], ["Shared folder", "共享文件夹", "共享資料夾"],
 ["On the sharing device", "在共享设备上", "在共享裝置上"], ["Files stay in the selected folder. Available actions follow its access settings.", "文件保留在所选文件夹中。可用操作由访问设置决定。", "檔案保留在所選資料夾中。可用操作由存取設定決定。"],
 ["Folder path", "文件夹路径", "資料夾路徑"], ["Shared files", "共享文件", "共享檔案"], ["Browse and transfer files from this device.", "浏览并传输此设备上的文件。", "瀏覽並傳輸此裝置上的檔案。"],
 ["Folder actions", "文件夹操作", "資料夾操作"], ["Refresh folder", "刷新文件夹", "重新整理資料夾"], ["Dismiss message", "关闭消息", "關閉訊息"],
 ["Selected {0}.", "已选择 {0}。", "已選取 {0}。"], ["Selection cleared.", "已清除选择。", "已清除選取。"],
 ["Folder unavailable", "文件夹不可用", "資料夾無法使用"], ["Could not open this folder", "无法打开此文件夹", "無法開啟此資料夾"], ["Retry folder", "重新打开文件夹", "重新開啟資料夾"],
 ["File browsing is unavailable", "文件浏览不可用", "檔案瀏覽無法使用"], ["The sharing device has not enabled file access.", "共享设备尚未启用文件访问。", "共享裝置尚未啟用檔案存取。"],
 ["Folder tools", "文件夹工具", "資料夾工具"], ["Selected entry", "所选项目", "所選項目"], ["Clear selection", "清除选择", "清除選取"],
 ["Size", "大小", "大小"], ["Modified", "修改时间", "修改時間"], ["Path", "路径", "路徑"], ["Download selected {0}", "下载所选 {0}", "下載所選 {0}"],
 ["Download", "下载", "下載"], ["Local file sharing", "本地文件共享", "本機檔案共享"], ["Can edit", "可编辑", "可編輯"], ["Read only", "只读", "唯讀"], ["Lock files", "锁定文件", "鎖定檔案"],
 ["Unlock shared files", "解锁共享文件", "解鎖共享檔案"], ["Enter the access key set on the device running {0}.", "输入运行 {0} 的设备上设置的访问密钥。", "輸入執行 {0} 的裝置上設定的存取金鑰。"],
 ["Access key", "访问密钥", "存取金鑰"], ["Unlocking…", "正在解锁…", "正在解鎖…"], ["Unlock files", "解锁文件", "解鎖檔案"],
 ["Folder contents", "文件夹内容", "資料夾內容"], ["Search this folder", "搜索此文件夹", "搜尋此資料夾"], ["Loading files…", "正在加载文件…", "正在載入檔案…"],
 ["{0} item", "{0} 项", "{0} 項"], ["{0} items", "{0} 项", "{0} 項"], ["Name", "名称", "名稱"], ["Open {0}", "打开 {0}", "開啟 {0}"], ["Select {0}", "选择 {0}", "選取 {0}"], ["Download {0}", "下载 {0}", "下載 {0}"], ["Details", "详情", "詳細資訊"],
 ["No matching files", "没有匹配的文件", "沒有相符的檔案"], ["This folder is empty", "此文件夹为空", "此資料夾是空的"], ["Try another name, or clear the search.", "尝试其他名称，或清除搜索。", "試試其他名稱，或清除搜尋。"],
 ["Upload a file or create a folder to get started.", "上传文件或创建文件夹以开始使用。", "上傳檔案或建立資料夾以開始使用。"], ["Upload a file to get started.", "上传文件以开始使用。", "上傳檔案以開始使用。"], ["Create a folder to get started.", "创建文件夹以开始使用。", "建立資料夾以開始使用。"], ["Files will appear here when the sharing device adds them.", "共享设备添加文件后，文件会显示在这里。", "共享裝置加入檔案後，檔案會顯示在這裡。"],
 ["Folder", "文件夹", "資料夾"], ["Unavailable", "不可用", "無法使用"], ["New folder", "新建文件夹", "新增資料夾"],
 ["Enter a name without slashes. Use Move to change folders.", "请输入不含斜杠的名称。使用移动操作更换文件夹。", "請輸入不含斜線的名稱。使用移動操作更換資料夾。"], ["Enter a destination relative to the shared folder.", "请输入相对于共享文件夹的目标路径。", "請輸入相對於共享資料夾的目標路徑。"],
 ["Folder created.", "已创建文件夹。", "已建立資料夾。"], ["File copied.", "已复制文件。", "已複製檔案。"], ["Entry updated.", "已更新项目。", "已更新項目。"], ["Entry deleted.", "已删除项目。", "已刪除項目。"],
 ["File actions", "文件操作", "檔案操作"], ["Folder name", "文件夹名称", "資料夾名稱"], ["In {0}", "位于 {0}", "位於 {0}"], ["the shared folder", "共享文件夹", "共享資料夾"],
 ["Creating…", "正在创建…", "正在建立…"], ["Create folder", "创建文件夹", "建立資料夾"], ["Cancel", "取消", "取消"], ["Rename", "重命名", "重新命名"], ["Move", "移动", "移動"], ["Copy", "复制", "複製"], ["Delete", "删除", "刪除"],
 ["New name", "新名称", "新名稱"], ["Destination path", "目标路径", "目標路徑"], ["Relative to the shared folder, including the file name. Existing files are preserved.", "相对于共享文件夹，包含文件名称。现有文件会被保留。", "相對於共享資料夾，包含檔案名稱。現有檔案會被保留。"],
 ["Working…", "正在处理…", "正在處理…"], ["Save name", "保存名称", "儲存名稱"], ["Move folder", "移动文件夹", "移動資料夾"], ["Move file", "移动文件", "移動檔案"], ["Copy folder", "复制文件夹", "複製資料夾"], ["Copy file", "复制文件", "複製檔案"],
 ["Delete {0}?", "删除 {0}？", "刪除 {0}？"], ["This permanently removes the folder from the sharing device.", "这会从共享设备永久删除该文件夹。", "這會從共享裝置永久刪除該資料夾。"], ["This permanently removes the file from the sharing device.", "这会从共享设备永久删除该文件。", "這會從共享裝置永久刪除該檔案。"],
 ["Include all files and subfolders", "包括所有文件和子文件夹", "包括所有檔案和子資料夾"], ["Deleting…", "正在删除…", "正在刪除…"], ["Delete permanently", "永久删除", "永久刪除"],
 ["Upload files", "上传文件", "上傳檔案"], ["Queued", "等待中", "等候中"], ["Uploading", "上传中", "上傳中"], ["Saving…", "正在保存…", "正在儲存…"], ["Saved", "已保存", "已儲存"], ["Failed", "失败", "失敗"], ["Canceled", "已取消", "已取消"],
 ["Transfers", "传输", "傳輸"], ["Clear finished", "清除已完成", "清除已完成"], ["Upload up to {0} per file. Files are saved after the device confirms the transfer.", "每个文件最多上传 {0}。设备确认传输后才会保存文件。", "每個檔案最多上傳 {0}。裝置確認傳輸後才會儲存檔案。"],
 ["Cancel {0}", "取消 {0}", "取消 {0}"], ["Upload progress for {0}", "{0} 的上传进度", "{0} 的上傳進度"], ["Sent. Waiting for the device to confirm it is saved.", "已发送。正在等待设备确认保存。", "已傳送。正在等候裝置確認儲存。"], ["Stopped sending. Unfinished files are removed by the device.", "已停止发送。设备会删除未完成的文件。", "已停止傳送。裝置會刪除未完成的檔案。"], ["This file exceeds the {0} upload limit.", "此文件超过 {0} 的上传限制。", "此檔案超過 {0} 的上傳限制。"],
 ["Share files", "共享文件", "共享檔案"], ["Share a folder", "共享文件夹", "共享資料夾"],
 ["Use this app’s storage to share files with another device on your local network.", "使用此应用的存储空间与本地网络中的其他设备共享文件。", "使用此應用程式的儲存空間與區域網路中的其他裝置共享檔案。"],
 ["Choose the files this device makes available, then open the file manager or connect from another device.", "选择此设备要共享的文件，然后打开文件管理器或从其他设备连接。", "選擇此裝置要共享的檔案，然後開啟檔案管理員或從其他裝置連線。"],
 ["Keep this app on screen. Sharing stops when it goes into the background. Return here and start sharing again.", "请保持此应用在前台。进入后台会停止共享。返回此处后重新开始共享。", "請保持此應用程式在前景。進入背景會停止共享。返回此處後重新開始共享。"],
 ["Sharing is active.", "共享已启动。", "共享已啟動。"], ["Sharing is stopped.", "共享已停止。", "共享已停止。"], ["Connecting…", "正在连接…", "正在連線…"], ["Connecting to native controls…", "正在连接本机控制…", "正在連線至原生控制…"],
 ["App storage", "应用存储", "應用程式儲存空間"], ["Files uploaded here stay in this app. Uninstalling removes them.", "上传的文件保留在此应用中。卸载应用会删除这些文件。", "上傳的檔案保留在此應用程式中。解除安裝會刪除這些檔案。"], ["No folder selected", "尚未选择文件夹", "尚未選取資料夾"], ["Choose folder", "选择文件夹", "選取資料夾"],
 ["Stop sharing to change the folder or access settings.", "停止共享后可更改文件夹或访问设置。", "停止共享後可更改資料夾或存取設定。"], ["Stop sharing to change access settings.", "停止共享后可更改访问设置。", "停止共享後可更改存取設定。"], ["Access settings", "访问设置", "存取設定"],
 ["Active settings are unavailable. Stop sharing to configure a new session.", "无法获取当前设置。停止共享后可配置新会话。", "無法取得目前設定。停止共享後可設定新工作階段。"], ["Share on the local network", "在本地网络共享", "在區域網路共享"],
 ["Other devices can connect using this device’s local network address.", "其他设备可使用此设备的本地网络地址连接。", "其他裝置可使用此裝置的區域網路位址連線。"], ["Only browsers on this device can connect.", "仅此设备上的浏览器可以连接。", "僅此裝置上的瀏覽器可以連線。"],
 ["Port", "端口", "連接埠"], ["Use 0 to choose an available port.", "输入 0 以选择可用端口。", "輸入 0 以選擇可用連接埠。"], ["Allow uploads", "允许上传", "允許上傳"], ["Allow file management", "允许文件管理", "允許檔案管理"],
 ["Required for uploads and file management; optional for read-only sharing.", "上传和文件管理需要访问密钥；只读共享可选择设置。", "上傳和檔案管理需要存取金鑰；唯讀共享可選擇設定。"], ["An access key is configured on this device. Stop sharing to enter or generate a new key.", "此设备已配置访问密钥。停止共享后可输入或生成新密钥。", "此裝置已設定存取金鑰。停止共享後可輸入或產生新金鑰。"], ["No access key is set for this sharing session.", "此共享会话未设置访问密钥。", "此共享工作階段未設定存取金鑰。"],
 ["Hide access key", "隐藏访问密钥", "隱藏存取金鑰"], ["Show access key", "显示访问密钥", "顯示存取金鑰"], ["Generate access key", "生成访问密钥", "產生存取金鑰"], ["Stop sharing", "停止共享", "停止共享"], ["Start sharing", "开始共享", "開始共享"],
 ["Sharing is active", "共享已启动", "共享已啟動"], ["Ready when you are", "准备就绪", "準備就緒"], ["Open the file manager on this device to browse and transfer files.", "打开此设备上的文件管理器以浏览和传输文件。", "開啟此裝置上的檔案管理員以瀏覽和傳輸檔案。"], ["Open a local network address on another device to upload, browse and manage files.", "在其他设备上打开本地网络地址以上传、浏览和管理文件。", "在其他裝置上開啟區域網路位址以上傳、瀏覽和管理檔案。"],
 ["Open file manager", "打开文件管理器", "開啟檔案管理員"], ["This device", "此设备", "此裝置"], ["Local network", "本地网络", "區域網路"],
 ["Share a local network address and the access key with the person connecting. Closing the file manager stops sharing.", "将本地网络地址和访问密钥提供给连接者。关闭文件管理器会停止共享。", "將區域網路位址和存取金鑰提供給連線者。關閉檔案管理員會停止共享。"], ["Share the address and any access key with the person connecting. Keep this app on screen while transferring.", "将地址和访问密钥提供给连接者。传输时请保持此应用在前台。", "將位址和存取金鑰提供給連線者。傳輸時請保持此應用程式在前景。"],
 ["Start sharing, then upload files from another device. Files stay in this app.", "开始共享后，从其他设备上传文件。文件保留在此应用中。", "開始共享後，從其他裝置上傳檔案。檔案保留在此應用程式中。"], ["Select a folder and start sharing. Files stay on this device.", "选择文件夹并开始共享。文件保留在此设备上。", "選取資料夾並開始共享。檔案保留在此裝置上。"], ["Loading local diagnostics…", "正在加载本地诊断…", "正在載入本機診斷…"],
 ["Cannot open the shared folder.", "无法打开共享文件夹。", "無法開啟共享資料夾。"], ["Could not list this folder.", "无法列出此文件夹内容。", "無法列出此資料夾內容。"], ["Could not lock the files. Try again.", "无法锁定文件。请重试。", "無法鎖定檔案。請重試。"], ["Could not unlock files. Check the access key.", "无法解锁文件。请检查访问密钥。", "無法解鎖檔案。請檢查存取金鑰。"],
 ["The action failed. Refresh and try again.", "操作失败。请刷新后重试。", "操作失敗。請重新整理後重試。"], ["Could not delete this entry.", "无法删除此项目。", "無法刪除此項目。"], ["Upload failed. Choose the file again to retry.", "上传失败。请重新选择文件以重试。", "上傳失敗。請重新選取檔案以重試。"], ["The native action failed. Try again.", "本机操作失败。请重试。", "原生操作失敗。請重試。"],
 ["Check the access key and unlock files again.", "请检查访问密钥并重新解锁文件。", "請檢查存取金鑰並重新解鎖檔案。"], ["This action is not allowed by the sharing device.", "共享设备不允许此操作。", "共享裝置不允許此操作。"], ["This path is invalid. Use a path inside the shared folder.", "路径无效。请使用共享文件夹内的路径。", "路徑無效。請使用共享資料夾內的路徑。"], ["The entry was not found. Refresh the folder.", "找不到该项目。请刷新文件夹。", "找不到該項目。請重新整理資料夾。"], ["The destination already exists or the entry is busy. Refresh and try again.", "目标已存在或项目正忙。请刷新后重试。", "目標已存在或項目忙碌中。請重新整理後重試。"], ["This operation is not supported by the sharing device.", "共享设备不支持此操作。", "共享裝置不支援此操作。"],
 ["The upload exceeds the device’s file size limit.", "上传超过设备的文件大小限制。", "上傳超過裝置的檔案大小限制。"], ["The request failed. Refresh the folder and try again.", "请求失败。请刷新文件夹后重试。", "請求失敗。請重新整理資料夾後重試。"], ["Cannot reach the sharing device. Check the connection and try again.", "无法连接共享设备。请检查连接后重试。", "無法連線至共享裝置。請檢查連線後重試。"], ["The server did not confirm that the file was saved.", "服务器未确认文件已保存。", "伺服器未確認檔案已儲存。"],
 ["The destination already exists. Choose another name.", "目标已存在。请选择其他名称。", "目標已存在。請選擇其他名稱。"],
 ["The folder is not empty. Confirm removal of all files and subfolders to delete it.", "文件夹不为空。请确认删除所有文件和子文件夹。", "資料夾不是空的。請確認刪除所有檔案和子資料夾。"],
 ["The requested byte range is unavailable. Start the download again.", "请求的字节范围不可用。请重新开始下载。", "請求的位元組範圍無法使用。請重新開始下載。"],
 ["The device could not complete the storage operation. Refresh and try again.", "设备无法完成存储操作。请刷新后重试。", "裝置無法完成儲存操作。請重新整理後重試。"],
] as const;
export type MessageKey = typeof messages[number][0];
export type Catalog = Record<MessageKey, string>;
export const catalogs = Object.fromEntries(locales.map((locale, index) => [locale, Object.fromEntries(messages.map(row => [row[0], row[index]]))])) as Record<Locale, Catalog>;
export interface MessageDescriptor { key: MessageKey; args?: readonly (string | number)[] }
export type PresentationMessage = string | MessageDescriptor | Error;
export const describe = (key: MessageKey, ...args: (string | number)[]): MessageDescriptor => ({ key, args });
export function interpolate(text: string, args: readonly (string | number)[] = []): string {
  return text.replace(/\{(\d+)\}/g, (token, index: string) => String(args[Number(index)] ?? token));
}
export function presentError(error: unknown, fallback: MessageKey): PresentationMessage {
  return typeof error === "string" || error instanceof Error ? error : describe(fallback);
}
