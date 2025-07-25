import { decode, encode } from '@msgpack/msgpack';

export enum MessageType {
  SubmitName,
  NumNames,
  StatePlaying,
  Names,
  MakeGuess,
  NameGuessed,
  StateSubmitting,
}

export type SubmitNameMessage = {
  type: MessageType.SubmitName;
  content: string;
};

export type NumNamesMessage = {
  type: MessageType.NumNames;
  content: number;
};

export type StatePlayingMessage = {
  type: MessageType.StatePlaying;
  content: null;
};

export type NamesMessage = {
  type: MessageType.Names;
  content: [string[], boolean[]];
};

export type MakeGuessMessage = {
  type: MessageType.MakeGuess;
  content: number;
};

export type NameGuessedMessage = {
  type: MessageType.NameGuessed;
  content: number;
};

export type StateSubmittingMessage = {
  type: MessageType.StateSubmitting;
  content: null;
};

export type Message =
  | SubmitNameMessage
  | NumNamesMessage
  | StatePlayingMessage
  | NamesMessage
  | MakeGuessMessage
  | NameGuessedMessage
  | StateSubmittingMessage;

function bitfieldToGuesses(bitfield: Uint8Array, numNames: number): boolean[] {
  const numBytesToProcess = Math.min(bitfield.length, Math.ceil(numNames / 8));
  const guesses: boolean[] = new Array(numBytesToProcess * 8);

  for (let i = 0; i < numBytesToProcess; i++) {
    const num = bitfield[i];
    const base = i * 8;
    guesses[base] = !!(num & 0x80);
    guesses[base + 1] = !!(num & 0x40);
    guesses[base + 2] = !!(num & 0x20);
    guesses[base + 3] = !!(num & 0x10);
    guesses[base + 4] = !!(num & 0x8);
    guesses[base + 5] = !!(num & 0x4);
    guesses[base + 6] = !!(num & 0x2);
    guesses[base + 7] = !!(num & 0x1);
  }

  if (guesses.length < numNames) {
    guesses.push(...new Array(numNames).fill(false));
  } else {
    guesses.splice(numNames);
  }

  return guesses;
}

export function decodeMessage(message: ArrayBuffer): Message {
  const view = new DataView(message);
  const type = view.getUint32(0);
  if (view.byteLength === 4) {
    return { type, content: null };
  }

  let content = decode(new Uint8Array(message, 4)) as Message['content'];
  if (type === MessageType.Names) {
    const [names, guessesBitfield] = content as unknown as [
      string[],
      Uint8Array,
    ];
    const guesses = bitfieldToGuesses(guessesBitfield, names.length);
    content = [names, guesses];
  }

  return { type, content };
}

function guessesToBitfield(guesses: boolean[]): Uint8Array {
  const bitfield = new Uint8Array(Math.ceil(guesses.length / 8));
  for (let i = 0; i < guesses.length; i++) {
    bitfield[Math.floor(i / 8)] |= Number(guesses[i]) << (7 - (i % 8));
  }
  return bitfield;
}

export function encodeMessage(message: Message): ArrayBuffer {
  let content: Uint8Array | null;
  switch (message.type) {
    case MessageType.Names:
      content = encode([
        message.content[0],
        guessesToBitfield(message.content[1]),
      ]);
      break;
    case MessageType.SubmitName:
    case MessageType.NumNames:
    case MessageType.MakeGuess:
    case MessageType.NameGuessed:
      content = encode(message.content);
      break;
    case MessageType.StatePlaying:
    case MessageType.StateSubmitting:
      content = null;
      break;
  }

  const encoded = new ArrayBuffer(4 + (content?.length ?? 0));
  const view = new DataView(encoded);
  view.setUint32(0, message.type);

  if (content !== null) {
    new Uint8Array(encoded, 4).set(content);
  }

  return encoded;
}
