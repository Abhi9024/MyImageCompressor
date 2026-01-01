using MedImgCompress.Error;

namespace MedImgCompress;

/// <summary>
/// Image data structure for compression.
/// </summary>
public class ImageData
{
    /// <summary>Image width in pixels.</summary>
    public int Width { get; set; }

    /// <summary>Image height in pixels.</summary>
    public int Height { get; set; }

    /// <summary>Bits per sample (typically 8 or 16 for medical images).</summary>
    public int BitsPerSample { get; set; }

    /// <summary>Samples per pixel (1 for grayscale, 3 for RGB).</summary>
    public int SamplesPerPixel { get; set; }

    /// <summary>Raw pixel data.</summary>
    public byte[] PixelData { get; set; } = Array.Empty<byte>();

    /// <summary>Photometric interpretation (e.g., "MONOCHROME2", "RGB").</summary>
    public string PhotometricInterpretation { get; set; } = string.Empty;

    /// <summary>Whether pixel values are signed.</summary>
    public bool IsSigned { get; set; }

    /// <summary>
    /// Create a new ImageData instance.
    /// </summary>
    public ImageData() { }

    /// <summary>
    /// Create a new ImageData instance with parameters.
    /// </summary>
    public ImageData(int width, int height, int bitsPerSample, int samplesPerPixel, byte[] pixelData)
    {
        Width = width;
        Height = height;
        BitsPerSample = bitsPerSample;
        SamplesPerPixel = samplesPerPixel;
        PixelData = pixelData;
    }

    /// <summary>
    /// Calculate the expected size of pixel data in bytes.
    /// </summary>
    public int ExpectedSize
    {
        get
        {
            int bytesPerSample = (BitsPerSample + 7) / 8;
            return Width * Height * SamplesPerPixel * bytesPerSample;
        }
    }

    /// <summary>
    /// Validate that pixel data size matches expected size.
    /// </summary>
    /// <exception cref="ImageDataException">Thrown when validation fails.</exception>
    public void Validate()
    {
        int expected = ExpectedSize;
        if (PixelData.Length != expected)
        {
            throw new ImageDataException(
                $"Pixel data size mismatch: expected {expected} bytes, got {PixelData.Length}");
        }
    }

    /// <summary>
    /// Try to validate the pixel data.
    /// </summary>
    /// <param name="error">Error message if validation fails.</param>
    /// <returns>True if valid, false otherwise.</returns>
    public bool TryValidate(out string? error)
    {
        int expected = ExpectedSize;
        if (PixelData.Length != expected)
        {
            error = $"Pixel data size mismatch: expected {expected} bytes, got {PixelData.Length}";
            return false;
        }
        error = null;
        return true;
    }
}
