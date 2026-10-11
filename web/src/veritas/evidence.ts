// Evidence photos: strip identifying metadata on the device before hashing.
//
// Phone photos usually carry EXIF (often with GPS coordinates) and may carry
// XMP or IPTC blocks. Veritas never stores the photo, only its hash, but the
// photo is shown if a price is disputed. So the file the hash commits to is a
// re-encoded copy containing pixels only. A structural check then refuses any
// output that still has a metadata segment, so a browser that kept metadata
// cannot slip it through.

export type MetadataFinding =
  | 'jpeg-exif'
  | 'jpeg-xmp'
  | 'jpeg-iptc'
  | 'jpeg-comment'
  | 'png-exif'
  | 'png-text'
  | 'png-time'
  | 'webp-exif'
  | 'webp-xmp'
  | 'unknown-format'

const ascii = (bytes: Uint8Array, start: number, length: number) =>
  String.fromCharCode(...bytes.subarray(start, start + length))

/** Lists metadata blocks that can identify a person or place. Pixels, colour
 *  profiles and format headers are not reported. */
export function findMetadata(bytes: Uint8Array): MetadataFinding[] {
  if (bytes[0] === 0xff && bytes[1] === 0xd8) return jpegFindings(bytes)
  if (ascii(bytes, 1, 3) === 'PNG') return pngFindings(bytes)
  if (ascii(bytes, 0, 4) === 'RIFF' && ascii(bytes, 8, 4) === 'WEBP') return webpFindings(bytes)
  return ['unknown-format']
}

function jpegFindings(bytes: Uint8Array): MetadataFinding[] {
  // Walk every marker segment, including those between scans: progressive
  // JPEGs may place APPn or COM segments after the first scan.
  const found = new Set<MetadataFinding>()
  let offset = 2
  while (offset + 2 <= bytes.length) {
    if (bytes[offset] !== 0xff) return [...found, 'unknown-format'] // malformed: fail closed
    const marker = bytes[offset + 1]
    if (marker === 0xff) {
      offset += 1 // fill byte before a marker
      continue
    }
    if (marker === 0xd9) break // end of image
    if (marker >= 0xd0 && marker <= 0xd7) {
      offset += 2
      continue
    }
    if (offset + 4 > bytes.length) return [...found, 'unknown-format']
    const length = (bytes[offset + 2] << 8) | bytes[offset + 3]
    const body = offset + 4
    if (marker === 0xe1) {
      if (ascii(bytes, body, 6) === 'Exif\0\0') found.add('jpeg-exif')
      else if (ascii(bytes, body, 28).startsWith('http://ns.adobe.com/xap/1.0/')) found.add('jpeg-xmp')
      else found.add('jpeg-exif')
    } else if (marker === 0xed) {
      found.add('jpeg-iptc')
    } else if (marker === 0xfe) {
      found.add('jpeg-comment')
    }
    offset += 2 + length
    if (marker === 0xda) offset = skipEntropyData(bytes, offset)
  }
  return [...found]
}

/** Skips entropy-coded scan data. Inside it, 0xFF is followed by 0x00 (a
 *  stuffed byte) or a restart marker; any other marker ends the scan. */
function skipEntropyData(bytes: Uint8Array, offset: number): number {
  while (offset + 1 < bytes.length) {
    if (bytes[offset] === 0xff) {
      const next = bytes[offset + 1]
      if (next !== 0x00 && !(next >= 0xd0 && next <= 0xd7)) return offset
    }
    offset += 1
  }
  return bytes.length
}

function pngFindings(bytes: Uint8Array): MetadataFinding[] {
  const found = new Set<MetadataFinding>()
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  let offset = 8
  while (offset + 8 <= bytes.length) {
    const length = view.getUint32(offset)
    const type = ascii(bytes, offset + 4, 4)
    if (type === 'eXIf') found.add('png-exif')
    if (type === 'tEXt' || type === 'iTXt' || type === 'zTXt') found.add('png-text')
    if (type === 'tIME') found.add('png-time')
    if (type === 'IEND') break
    offset += 12 + length
  }
  return [...found]
}

function webpFindings(bytes: Uint8Array): MetadataFinding[] {
  const found = new Set<MetadataFinding>()
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  let offset = 12
  while (offset + 8 <= bytes.length) {
    const type = ascii(bytes, offset, 4)
    const length = view.getUint32(offset + 4, true)
    if (type === 'EXIF') found.add('webp-exif')
    if (type === 'XMP ') found.add('webp-xmp')
    offset += 8 + length + (length % 2)
  }
  return [...found]
}

/** Re-encodes an image to JPEG with pixels only and verifies the result has
 *  no metadata. The returned file is what the evidence hash must commit to;
 *  the attester keeps this file, not the original. */
export async function stripImageMetadata(file: Blob): Promise<Blob> {
  let bitmap: ImageBitmap
  try {
    bitmap = await createImageBitmap(file)
  } catch {
    throw new Error('This browser cannot read that image format. Try a JPEG or PNG.')
  }
  const canvas = document.createElement('canvas')
  canvas.width = bitmap.width
  canvas.height = bitmap.height
  const context = canvas.getContext('2d')
  if (!context) throw new Error('This browser cannot re-encode images.')
  context.drawImage(bitmap, 0, 0)
  bitmap.close()
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/jpeg', 0.92))
  if (!blob) throw new Error('The image could not be re-encoded.')
  const leftovers = findMetadata(new Uint8Array(await blob.arrayBuffer()))
  if (leftovers.length > 0) {
    throw new Error(`The cleaned image still contains metadata (${leftovers.join(', ')}); it was not used.`)
  }
  return blob
}
