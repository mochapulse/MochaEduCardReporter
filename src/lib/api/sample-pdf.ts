/**
 * Builds a minimal, valid one-page A4 PDF (Helvetica, one line of text).
 * Used by the mock API so the viewer has real bytes to show.
 */
export function buildSamplePdf(line: string): ArrayBuffer {
  const escaped = line
    .replace(/[^\x20-\x7e]/g, '?')
    .replace(/\\/g, '\\\\')
    .replace(/\(/g, '\\(')
    .replace(/\)/g, '\\)')
  const content = `BT /F1 24 Tf 72 760 Td (${escaped}) Tj ET`
  const objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>',
    `<< /Length ${String(content.length)} >>\nstream\n${content}\nendstream`,
    '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
  ]
  let body = '%PDF-1.4\n'
  const offsets: number[] = []
  objects.forEach((object, index) => {
    offsets.push(body.length)
    body += `${String(index + 1)} 0 obj\n${object}\nendobj\n`
  })
  const xrefAt = body.length
  body += `xref\n0 ${String(objects.length + 1)}\n0000000000 65535 f \n`
  for (const offset of offsets) body += `${String(offset).padStart(10, '0')} 00000 n \n`
  body += `trailer\n<< /Size ${String(objects.length + 1)} /Root 1 0 R >>\nstartxref\n${String(xrefAt)}\n%%EOF\n`
  const bytes = new Uint8Array(body.length)
  for (let i = 0; i < body.length; i += 1) bytes[i] = body.charCodeAt(i)
  return bytes.buffer
}
