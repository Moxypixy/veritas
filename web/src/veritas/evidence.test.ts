import { describe, expect, it } from 'vitest'

import { findMetadata } from './evidence'

const bytes = (...parts: (number[] | string)[]) =>
  Uint8Array.from(parts.flatMap((part) => (typeof part === 'string' ? [...part].map((c) => c.charCodeAt(0)) : part)))

/** A JPEG segment: marker, big-endian length (includes itself), payload. */
const segment = (marker: number, payload: number[] | string) => {
  const body = typeof payload === 'string' ? [...payload].map((c) => c.charCodeAt(0)) : payload
  const length = body.length + 2
  return [0xff, marker, length >> 8, length & 0xff, ...body]
}

const SOI = [0xff, 0xd8]
const SOS = [0xff, 0xda, 0x00, 0x02]
const JFIF = segment(0xe0, 'JFIF\0\x01\x01\0\0\x01\0\x01\0\0')

/** A PNG chunk: big-endian length, type, data, 4 CRC bytes (not checked). */
const chunk = (type: string, data: number[] = []) => [
  (data.length >>> 24) & 0xff,
  (data.length >>> 16) & 0xff,
  (data.length >>> 8) & 0xff,
  data.length & 0xff,
  ...[...type].map((c) => c.charCodeAt(0)),
  ...data,
  0,
  0,
  0,
  0,
]
const PNG_SIG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]

describe('findMetadata', () => {
  it('accepts a plain JPEG with only a JFIF header', () => {
    expect(findMetadata(bytes(SOI, JFIF, SOS))).toEqual([])
  })

  it('flags EXIF, XMP, IPTC and comments in a JPEG', () => {
    const jpeg = bytes(
      SOI,
      segment(0xe1, 'Exif\0\0MM\0*'),
      segment(0xe1, 'http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>'),
      segment(0xed, 'Photoshop 3.0\0'),
      segment(0xfe, 'shot at home'),
      SOS,
    )
    expect(findMetadata(jpeg).sort()).toEqual(['jpeg-comment', 'jpeg-exif', 'jpeg-iptc', 'jpeg-xmp'])
  })

  it('stops at the start of scan, so image data is never misread', () => {
    // Bytes after SOS look like an EXIF segment but are compressed pixels.
    expect(findMetadata(bytes(SOI, JFIF, SOS, segment(0xe1, 'Exif\0\0')))).toEqual([])
  })

  it('flags PNG eXIf, text and time chunks', () => {
    const png = bytes(PNG_SIG, chunk('IHDR', Array(13).fill(0)), chunk('eXIf', [1]), chunk('tEXt', [65]), chunk('tIME', Array(7).fill(0)), chunk('IEND'))
    expect(findMetadata(png).sort()).toEqual(['png-exif', 'png-text', 'png-time'])
    expect(findMetadata(bytes(PNG_SIG, chunk('IHDR', Array(13).fill(0)), chunk('IEND')))).toEqual([])
  })

  it('treats unknown formats as unsafe', () => {
    expect(findMetadata(bytes('GIF89a'))).toEqual(['unknown-format'])
  })
})
