export function uuidsEqual(left: Uint8Array, right: Uint8Array): boolean {
  const leftView = new DataView(left.buffer, left.byteOffset, left.byteLength);
  const rightView = new DataView(
    right.buffer,
    right.byteOffset,
    right.byteLength,
  );

  for (let i = 0; i < 16; i += 4) {
    if (leftView.getInt32(i) !== rightView.getInt32(i)) {
      return false;
    }
  }

  return true;
}
