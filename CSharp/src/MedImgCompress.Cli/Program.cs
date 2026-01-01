using System.CommandLine;
using MedImgCompress;
using MedImgCompress.Codec;
using MedImgCompress.Config;
using MedImgCompress.Dicom;
using MedImgCompress.Pipeline;

namespace MedImgCompress.Cli;

class Program
{
    static async Task<int> Main(string[] args)
    {
        var rootCommand = new RootCommand("Medical Image Compression Tool");

        // Compress command
        var compressCommand = CreateCompressCommand();
        rootCommand.AddCommand(compressCommand);

        // Info command
        var infoCommand = CreateInfoCommand();
        rootCommand.AddCommand(infoCommand);

        // Analyze command
        var analyzeCommand = CreateAnalyzeCommand();
        rootCommand.AddCommand(analyzeCommand);

        return await rootCommand.InvokeAsync(args);
    }

    static Command CreateCompressCommand()
    {
        var inputArg = new Argument<FileInfo>("input", "Input DICOM file");
        var outputArg = new Argument<FileInfo>("output", "Output DICOM file");

        var codecOption = new Option<string>(
            aliases: new[] { "-c", "--codec" },
            description: "Compression codec (jpeg2000, jpegls, uncompressed)",
            getDefaultValue: () => "jpeg2000");

        var modeOption = new Option<string>(
            aliases: new[] { "-m", "--mode" },
            description: "Compression mode (lossless, lossy, nearlossless)",
            getDefaultValue: () => "lossless");

        var ratioOption = new Option<float?>(
            aliases: new[] { "-r", "--ratio" },
            description: "Target compression ratio for lossy mode");

        var nearOption = new Option<byte?>(
            aliases: new[] { "-n", "--near" },
            description: "Near-lossless error tolerance (0-255)");

        var command = new Command("compress", "Compress a DICOM file")
        {
            inputArg,
            outputArg,
            codecOption,
            modeOption,
            ratioOption,
            nearOption
        };

        command.SetHandler((input, output, codec, mode, ratio, near) =>
        {
            try
            {
                var codecType = ParseCodec(codec);
                var compressionMode = ParseMode(mode);

                var config = new CompressionConfig
                {
                    Codec = codecType,
                    Mode = compressionMode,
                    TargetRatio = ratio,
                    NearLosslessError = near ?? 0
                };

                var pipeline = new CompressionPipeline(config);
                var result = pipeline.Compress(input.FullName, output.FullName);

                Console.WriteLine("Compression complete!");
                Console.WriteLine(result);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error: {ex.Message}");
                Environment.ExitCode = 1;
            }
        }, inputArg, outputArg, codecOption, modeOption, ratioOption, nearOption);

        return command;
    }

    static Command CreateInfoCommand()
    {
        var inputArg = new Argument<FileInfo>("input", "Input DICOM file");

        var command = new Command("info", "Display DICOM file information")
        {
            inputArg
        };

        command.SetHandler((input) =>
        {
            try
            {
                var dicom = DicomFile.Open(input.FullName);

                Console.WriteLine($"File: {input.Name}");
                Console.WriteLine($"SOP Class: {dicom.SopClassUid}");
                Console.WriteLine($"Modality: {dicom.Modality}");
                Console.WriteLine($"Transfer Syntax: {dicom.TransferSyntaxUid}");
                Console.WriteLine();
                Console.WriteLine("Image Properties:");
                Console.WriteLine($"  Dimensions: {dicom.Columns} x {dicom.Rows}");
                Console.WriteLine($"  Bits Allocated: {dicom.BitsAllocated}");
                Console.WriteLine($"  Bits Stored: {dicom.BitsStored}");
                Console.WriteLine($"  High Bit: {dicom.HighBit}");
                Console.WriteLine($"  Samples/Pixel: {dicom.SamplesPerPixel}");
                Console.WriteLine($"  Photometric: {dicom.PhotometricInterpretation}");
                Console.WriteLine($"  Pixel Representation: {(dicom.PixelRepresentation == 0 ? "Unsigned" : "Signed")}");
                Console.WriteLine($"  Pixel Data Size: {FormatSize(dicom.PixelData.Length)}");
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error: {ex.Message}");
                Environment.ExitCode = 1;
            }
        }, inputArg);

        return command;
    }

    static Command CreateAnalyzeCommand()
    {
        var inputArg = new Argument<FileInfo>("input", "Input DICOM file");

        var command = new Command("analyze", "Analyze compression potential")
        {
            inputArg
        };

        command.SetHandler((input) =>
        {
            try
            {
                var dicom = DicomFile.Open(input.FullName);
                var imageData = dicom.GetImageData();

                Console.WriteLine($"Analyzing: {input.Name}");
                Console.WriteLine();

                // Test each codec
                var codecs = new[] { CompressionCodec.Jpeg2000, CompressionCodec.JpegLs };

                foreach (var codecType in codecs)
                {
                    var codec = CodecFactory.Create(codecType);

                    if (!codec.CanEncode(imageData))
                    {
                        Console.WriteLine($"{codec.Info.Name}: Not supported for this image");
                        continue;
                    }

                    Console.WriteLine($"{codec.Info.Name}:");

                    // Lossless
                    var losslessConfig = CompressionConfig.Lossless(codecType);
                    var losslessData = codec.Encode(imageData, losslessConfig);
                    double losslessRatio = (double)imageData.PixelData.Length / losslessData.Length;
                    Console.WriteLine($"  Lossless: {losslessRatio:F2}:1 ({losslessData.Length} bytes)");

                    // Lossy (if supported)
                    if (codec.Info.SupportsLossy)
                    {
                        var lossyConfig = CompressionConfig.Lossy(codecType, 10.0f);
                        var lossyData = codec.Encode(imageData, lossyConfig);
                        double lossyRatio = (double)imageData.PixelData.Length / lossyData.Length;
                        Console.WriteLine($"  Lossy (10:1 target): {lossyRatio:F2}:1 ({lossyData.Length} bytes)");
                    }

                    Console.WriteLine();
                }

                Console.WriteLine("Recommendations:");
                Console.WriteLine($"  Modality: {dicom.Modality}");

                var modality = ModalityExtensions.FromDicomCode(dicom.Modality);
                var preset = modality.GetDefaultPreset();
                Console.WriteLine($"  Suggested preset: {preset}");

                if (modality.RequiresLossless())
                {
                    Console.WriteLine("  Note: This modality requires lossless compression for compliance");
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error: {ex.Message}");
                Environment.ExitCode = 1;
            }
        }, inputArg);

        return command;
    }

    static CompressionCodec ParseCodec(string codec)
    {
        return codec.ToLowerInvariant() switch
        {
            "jpeg2000" or "j2k" => CompressionCodec.Jpeg2000,
            "jpegls" or "jls" => CompressionCodec.JpegLs,
            "uncompressed" or "raw" => CompressionCodec.Uncompressed,
            _ => throw new ArgumentException($"Unknown codec: {codec}")
        };
    }

    static CompressionMode ParseMode(string mode)
    {
        return mode.ToLowerInvariant() switch
        {
            "lossless" => CompressionMode.Lossless,
            "lossy" => CompressionMode.Lossy,
            "nearlossless" or "near-lossless" => CompressionMode.NearLossless,
            _ => throw new ArgumentException($"Unknown mode: {mode}")
        };
    }

    static string FormatSize(long bytes)
    {
        string[] units = { "B", "KB", "MB", "GB" };
        double size = bytes;
        int unit = 0;

        while (size >= 1024 && unit < units.Length - 1)
        {
            size /= 1024;
            unit++;
        }

        return $"{size:F2} {units[unit]}";
    }
}
