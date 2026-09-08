using System;
using System.Text;
using Windows.Data.Pdf;
using Windows.Globalization;
using Windows.Graphics.Imaging;
using Windows.Media.Ocr;
using Windows.Storage;
using Windows.Storage.Streams;

class Program {
  static int Main(string[] args) {
    if (args.Length < 1) {
      Console.Error.WriteLine("usage: PdfOcr <file.pdf>");
      return 2;
    }
    try {
      Console.OutputEncoding = new UTF8Encoding(false);
      Console.Write(Run(args[0]));
      return 0;
    } catch (Exception e) {
      Console.Error.WriteLine(e.ToString());
      return 1;
    }
  }

  static string Run(string path) {
    var file = System.WindowsRuntimeSystemExtensions.AsTask(StorageFile.GetFileFromPathAsync(path)).Result;
    var pdf = System.WindowsRuntimeSystemExtensions.AsTask(PdfDocument.LoadFromFileAsync(file)).Result;
    var ocr = OcrEngine.TryCreateFromUserProfileLanguages()
      ?? OcrEngine.TryCreateFromLanguage(new Language("pt-PT"))
      ?? OcrEngine.TryCreateFromLanguage(new Language("en"));
    if (ocr == null) throw new Exception("OCR engine unavailable");
    var sb = new StringBuilder();
    for (uint i = 0; i < pdf.PageCount; i++) {
      var page = pdf.GetPage(i);
      using (var stream = new InMemoryRandomAccessStream()) {
        System.WindowsRuntimeSystemExtensions.AsTask(page.RenderToStreamAsync(stream)).Wait();
        stream.Seek(0);
        var dec = System.WindowsRuntimeSystemExtensions.AsTask(BitmapDecoder.CreateAsync(stream)).Result;
        var bmp = System.WindowsRuntimeSystemExtensions.AsTask(dec.GetSoftwareBitmapAsync()).Result;
        var r = System.WindowsRuntimeSystemExtensions.AsTask(ocr.RecognizeAsync(bmp)).Result;
        sb.AppendLine(r.Text);
      }
      page.Close();
    }
    return sb.ToString();
  }
}
