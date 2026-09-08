param([Parameter(Mandatory=$true)][string]$PdfPath)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null
$asTaskAction = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
  $_.Name -eq 'AsTask' -and -not $_.IsGenericMethod -and $_.GetParameters().Count -eq 1
} | Select-Object -First 1
$asTaskOp = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
  $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
} | Select-Object -First 1
function Await-Action($op) {
  $asTaskAction.Invoke($null, @($op)).GetAwaiter().GetResult()
}
function Await-Op($op) {
  $t = $op.GetType().GetGenericArguments()[0]
  $asTaskOp.MakeGenericMethod($t).Invoke($null, @($op)).GetAwaiter().GetResult()
}
[void][Windows.Storage.StorageFile,Windows.Storage,ContentType=WindowsRuntime]
[void][Windows.Data.Pdf.PdfDocument,Windows.Data.Pdf,ContentType=WindowsRuntime]
[void][Windows.Media.Ocr.OcrEngine,Windows.Media.Ocr,ContentType=WindowsRuntime]
[void][Windows.Graphics.Imaging.BitmapDecoder,Windows.Graphics.Imaging,ContentType=WindowsRuntime]
[void][Windows.Storage.Streams.InMemoryRandomAccessStream,Windows.Storage.Streams,ContentType=WindowsRuntime]
$full = (Resolve-Path -LiteralPath $PdfPath).Path
$file = Await-Op ([Windows.Storage.StorageFile]::GetFileFromPathAsync($full))
$pdf = Await-Op ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file))
$ocr = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if (-not $ocr) {
  $ocr = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage([Windows.Globalization.Language]::new('en'))
}
$sb = New-Object System.Text.StringBuilder
for ($i = 0; $i -lt $pdf.PageCount; $i++) {
  $page = $pdf.GetPage($i)
  $stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
  Await-Action ($page.RenderToStreamAsync($stream))
  $stream.Seek(0) | Out-Null
  $decoder = Await-Op ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream))
  $bmp = Await-Op ($decoder.GetSoftwareBitmapAsync())
  $result = Await-Op ($ocr.RecognizeAsync($bmp))
  [void]$sb.AppendLine($result.Text)
  $page.Close()
}
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false
[Console]::Write($sb.ToString())
