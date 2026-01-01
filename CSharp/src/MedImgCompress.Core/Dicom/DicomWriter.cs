using System.Text;
using MedImgCompress.Codec;
using MedImgCompress.Config;

namespace MedImgCompress.Dicom;

/// <summary>
/// DICOM file writer for creating compressed DICOM files.
/// </summary>
public class DicomWriter
{
    private readonly MemoryStream _stream;
    private readonly bool _littleEndian = true;

    /// <summary>
    /// Create a new DICOM writer.
    /// </summary>
    public DicomWriter()
    {
        _stream = new MemoryStream();
    }

    /// <summary>
    /// Write a compressed DICOM file.
    /// </summary>
    public byte[] Write(DicomFile source, byte[] compressedData, string transferSyntaxUid)
    {
        _stream.SetLength(0);
        _stream.Position = 0;

        // Write preamble (128 bytes of zeros)
        _stream.Write(new byte[128]);

        // Write DICM prefix
        _stream.Write(Encoding.ASCII.GetBytes("DICM"));

        // File Meta Information
        WriteFileMetaInfo(source, transferSyntaxUid);

        // Copy source elements (except pixel data) and write new pixel data
        WriteDataset(source, compressedData);

        return _stream.ToArray();
    }

    private void WriteFileMetaInfo(DicomFile source, string transferSyntaxUid)
    {
        // File Meta Information Group Length - placeholder, will update
        long lengthPos = _stream.Position;
        WriteElement(0x00020000, "UL", BitConverter.GetBytes(0u));

        long metaStart = _stream.Position;

        // File Meta Information Version
        WriteElement(0x00020001, "OB", new byte[] { 0x00, 0x01 });

        // Media Storage SOP Class UID
        WriteElement(0x00020002, "UI", PadString(source.SopClassUid));

        // Media Storage SOP Instance UID
        WriteElement(0x00020003, "UI", PadString(source.SopInstanceUid));

        // Transfer Syntax UID
        WriteElement(0x00020010, "UI", PadString(transferSyntaxUid));

        // Implementation Class UID
        WriteElement(0x00020012, "UI", PadString("1.2.826.0.1.3680043.10.1.1"));

        // Implementation Version Name
        WriteElement(0x00020013, "SH", PadString("MEDIMGCOMPRESS"));

        // Update group length
        long metaEnd = _stream.Position;
        int metaLength = (int)(metaEnd - metaStart);
        _stream.Position = lengthPos + 8; // Skip tag and VR/length
        _stream.Write(BitConverter.GetBytes((uint)metaLength));
        _stream.Position = metaEnd;
    }

    private void WriteDataset(DicomFile source, byte[] compressedData)
    {
        // Write basic image attributes
        WriteElement(DicomTags.SopClassUid, "UI", PadString(source.SopClassUid));
        WriteElement(DicomTags.SopInstanceUid, "UI", PadString(source.SopInstanceUid));
        WriteElement(DicomTags.Modality, "CS", PadString(source.Modality));

        // Image Pixel Module
        WriteElement(DicomTags.SamplesPerPixel, "US",
            BitConverter.GetBytes((ushort)source.SamplesPerPixel));
        WriteElement(DicomTags.PhotometricInterpretation, "CS",
            PadString(source.PhotometricInterpretation));
        WriteElement(DicomTags.Rows, "US",
            BitConverter.GetBytes((ushort)source.Rows));
        WriteElement(DicomTags.Columns, "US",
            BitConverter.GetBytes((ushort)source.Columns));
        WriteElement(DicomTags.BitsAllocated, "US",
            BitConverter.GetBytes((ushort)source.BitsAllocated));
        WriteElement(DicomTags.BitsStored, "US",
            BitConverter.GetBytes((ushort)source.BitsStored));
        WriteElement(DicomTags.HighBit, "US",
            BitConverter.GetBytes((ushort)source.HighBit));
        WriteElement(DicomTags.PixelRepresentation, "US",
            BitConverter.GetBytes((ushort)source.PixelRepresentation));

        // Write compressed pixel data
        WritePixelData(compressedData);
    }

    private void WritePixelData(byte[] data)
    {
        // Write pixel data tag
        ushort group = DicomTags.GetGroup(DicomTags.PixelData);
        ushort element = DicomTags.GetElement(DicomTags.PixelData);
        WriteUInt16(group);
        WriteUInt16(element);

        // Write VR and length for encapsulated pixel data
        _stream.Write(Encoding.ASCII.GetBytes("OB"));
        _stream.Write(new byte[] { 0x00, 0x00 }); // Reserved
        _stream.Write(BitConverter.GetBytes(0xFFFFFFFF)); // Undefined length

        // Write basic offset table (empty)
        WriteUInt16(0xFFFE);
        WriteUInt16(0xE000);
        _stream.Write(BitConverter.GetBytes(0u)); // Zero length

        // Write fragment
        WriteUInt16(0xFFFE);
        WriteUInt16(0xE000);
        _stream.Write(BitConverter.GetBytes((uint)data.Length));
        _stream.Write(data);

        // Pad to even length if necessary
        if (data.Length % 2 != 0)
            _stream.WriteByte(0);

        // Write sequence delimiter
        WriteUInt16(0xFFFE);
        WriteUInt16(0xE0DD);
        _stream.Write(BitConverter.GetBytes(0u));
    }

    private void WriteElement(uint tag, string vr, byte[] value)
    {
        ushort group = DicomTags.GetGroup(tag);
        ushort element = DicomTags.GetElement(tag);

        WriteUInt16(group);
        WriteUInt16(element);
        _stream.Write(Encoding.ASCII.GetBytes(vr));

        // VRs with 32-bit length
        if (vr == "OB" || vr == "OD" || vr == "OF" || vr == "OL" || vr == "OW" ||
            vr == "SQ" || vr == "UC" || vr == "UN" || vr == "UR" || vr == "UT")
        {
            _stream.Write(new byte[] { 0x00, 0x00 }); // Reserved
            _stream.Write(BitConverter.GetBytes((uint)value.Length));
        }
        else
        {
            WriteUInt16((ushort)value.Length);
        }

        _stream.Write(value);
    }

    private void WriteUInt16(ushort value)
    {
        if (_littleEndian)
        {
            _stream.WriteByte((byte)(value & 0xFF));
            _stream.WriteByte((byte)(value >> 8));
        }
        else
        {
            _stream.WriteByte((byte)(value >> 8));
            _stream.WriteByte((byte)(value & 0xFF));
        }
    }

    private static byte[] PadString(string value)
    {
        // Pad to even length with space for non-UI, null for UI
        string padded = value.Length % 2 != 0 ? value + "\0" : value;
        return Encoding.ASCII.GetBytes(padded);
    }
}
