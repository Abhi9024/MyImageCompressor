using System.Text;
using MedImgCompress.Error;

namespace MedImgCompress.Dicom;

/// <summary>
/// DICOM file parser for extracting image data and metadata.
/// </summary>
public class DicomFile
{
    private readonly Dictionary<uint, byte[]> _elements = new();
    private readonly bool _explicitVr;
    private readonly bool _littleEndian;

    /// <summary>
    /// File path of the DICOM file.
    /// </summary>
    public string FilePath { get; }

    /// <summary>
    /// Transfer Syntax UID.
    /// </summary>
    public string TransferSyntaxUid { get; private set; } = string.Empty;

    /// <summary>
    /// SOP Class UID.
    /// </summary>
    public string SopClassUid { get; private set; } = string.Empty;

    /// <summary>
    /// SOP Instance UID.
    /// </summary>
    public string SopInstanceUid { get; private set; } = string.Empty;

    /// <summary>
    /// Image modality.
    /// </summary>
    public string Modality { get; private set; } = string.Empty;

    /// <summary>
    /// Image width in pixels.
    /// </summary>
    public int Columns { get; private set; }

    /// <summary>
    /// Image height in pixels.
    /// </summary>
    public int Rows { get; private set; }

    /// <summary>
    /// Bits allocated per sample.
    /// </summary>
    public int BitsAllocated { get; private set; }

    /// <summary>
    /// Bits stored per sample.
    /// </summary>
    public int BitsStored { get; private set; }

    /// <summary>
    /// High bit position.
    /// </summary>
    public int HighBit { get; private set; }

    /// <summary>
    /// Pixel representation (0 = unsigned, 1 = signed).
    /// </summary>
    public int PixelRepresentation { get; private set; }

    /// <summary>
    /// Samples per pixel.
    /// </summary>
    public int SamplesPerPixel { get; private set; } = 1;

    /// <summary>
    /// Photometric interpretation.
    /// </summary>
    public string PhotometricInterpretation { get; private set; } = "MONOCHROME2";

    /// <summary>
    /// Raw pixel data.
    /// </summary>
    public byte[] PixelData { get; private set; } = Array.Empty<byte>();

    private DicomFile(string filePath, bool explicitVr, bool littleEndian)
    {
        FilePath = filePath;
        _explicitVr = explicitVr;
        _littleEndian = littleEndian;
    }

    /// <summary>
    /// Open and parse a DICOM file.
    /// </summary>
    public static DicomFile Open(string filePath)
    {
        if (!File.Exists(filePath))
            throw new DicomException($"File not found: {filePath}");

        byte[] data = File.ReadAllBytes(filePath);
        return Parse(data, filePath);
    }

    /// <summary>
    /// Parse DICOM data from bytes.
    /// </summary>
    public static DicomFile Parse(byte[] data, string filePath = "memory")
    {
        if (data.Length < 132)
            throw new DicomException("Invalid DICOM file: too short");

        // Check for DICM prefix at offset 128
        if (data[128] != 'D' || data[129] != 'I' || data[130] != 'C' || data[131] != 'M')
            throw new DicomException("Invalid DICOM file: missing DICM prefix");

        var file = new DicomFile(filePath, explicitVr: true, littleEndian: true);
        file.ParseElements(data, 132);

        return file;
    }

    private void ParseElements(byte[] data, int offset)
    {
        int pos = offset;

        while (pos < data.Length - 4)
        {
            // Read tag
            ushort group = ReadUInt16(data, pos, _littleEndian);
            ushort element = ReadUInt16(data, pos + 2, _littleEndian);
            uint tag = DicomTags.MakeTag(group, element);
            pos += 4;

            // Determine VR and length
            string vr = "";
            int length;

            if (_explicitVr && group != 0xFFFE)
            {
                if (pos + 2 > data.Length) break;
                vr = Encoding.ASCII.GetString(data, pos, 2);
                pos += 2;

                // Check for VRs with 32-bit length
                if (vr == "OB" || vr == "OD" || vr == "OF" || vr == "OL" || vr == "OW" ||
                    vr == "SQ" || vr == "UC" || vr == "UN" || vr == "UR" || vr == "UT")
                {
                    pos += 2; // Skip reserved bytes
                    if (pos + 4 > data.Length) break;
                    length = (int)ReadUInt32(data, pos, _littleEndian);
                    pos += 4;
                }
                else
                {
                    if (pos + 2 > data.Length) break;
                    length = ReadUInt16(data, pos, _littleEndian);
                    pos += 2;
                }
            }
            else
            {
                if (pos + 4 > data.Length) break;
                length = (int)ReadUInt32(data, pos, _littleEndian);
                pos += 4;
            }

            // Handle undefined length
            if (length == -1 || length == unchecked((int)0xFFFFFFFF))
            {
                // Skip sequences with undefined length for now
                continue;
            }

            if (length < 0 || pos + length > data.Length)
                break;

            // Extract value
            byte[] value = new byte[length];
            Array.Copy(data, pos, value, 0, length);
            _elements[tag] = value;
            pos += length;

            // Parse known elements
            ParseElement(tag, value);
        }
    }

    private void ParseElement(uint tag, byte[] value)
    {
        switch (tag)
        {
            case DicomTags.TransferSyntaxUid:
                TransferSyntaxUid = GetString(value);
                break;
            case DicomTags.SopClassUid:
                SopClassUid = GetString(value);
                break;
            case DicomTags.SopInstanceUid:
                SopInstanceUid = GetString(value);
                break;
            case DicomTags.Modality:
                Modality = GetString(value);
                break;
            case DicomTags.Rows:
                Rows = GetUInt16(value);
                break;
            case DicomTags.Columns:
                Columns = GetUInt16(value);
                break;
            case DicomTags.BitsAllocated:
                BitsAllocated = GetUInt16(value);
                break;
            case DicomTags.BitsStored:
                BitsStored = GetUInt16(value);
                break;
            case DicomTags.HighBit:
                HighBit = GetUInt16(value);
                break;
            case DicomTags.PixelRepresentation:
                PixelRepresentation = GetUInt16(value);
                break;
            case DicomTags.SamplesPerPixel:
                SamplesPerPixel = GetUInt16(value);
                break;
            case DicomTags.PhotometricInterpretation:
                PhotometricInterpretation = GetString(value);
                break;
            case DicomTags.PixelData:
                PixelData = value;
                break;
        }
    }

    /// <summary>
    /// Get image data from the DICOM file.
    /// </summary>
    public ImageData GetImageData()
    {
        if (PixelData.Length == 0)
            throw new DicomException("No pixel data found in DICOM file");

        return new ImageData
        {
            Width = Columns,
            Height = Rows,
            BitsPerSample = BitsStored > 0 ? BitsStored : BitsAllocated,
            SamplesPerPixel = SamplesPerPixel,
            PixelData = PixelData,
            PhotometricInterpretation = PhotometricInterpretation,
            IsSigned = PixelRepresentation == 1
        };
    }

    /// <summary>
    /// Get a string value from an element.
    /// </summary>
    public string? GetString(uint tag)
    {
        return _elements.TryGetValue(tag, out var value) ? GetString(value) : null;
    }

    /// <summary>
    /// Get an integer value from an element.
    /// </summary>
    public int? GetInt(uint tag)
    {
        if (!_elements.TryGetValue(tag, out var value)) return null;
        return value.Length >= 2 ? GetUInt16(value) : null;
    }

    private static string GetString(byte[] value)
    {
        string s = Encoding.ASCII.GetString(value);
        return s.TrimEnd('\0', ' ');
    }

    private int GetUInt16(byte[] value)
    {
        return value.Length >= 2 ? ReadUInt16(value, 0, _littleEndian) : 0;
    }

    private static ushort ReadUInt16(byte[] data, int offset, bool littleEndian)
    {
        if (littleEndian)
            return (ushort)(data[offset] | (data[offset + 1] << 8));
        return (ushort)((data[offset] << 8) | data[offset + 1]);
    }

    private static uint ReadUInt32(byte[] data, int offset, bool littleEndian)
    {
        if (littleEndian)
            return (uint)(data[offset] | (data[offset + 1] << 8) |
                         (data[offset + 2] << 16) | (data[offset + 3] << 24));
        return (uint)((data[offset] << 24) | (data[offset + 1] << 16) |
                     (data[offset + 2] << 8) | data[offset + 3]);
    }
}
