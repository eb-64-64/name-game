import { decode, encode } from '@msgpack/msgpack';
import { parse, stringify } from 'uuid';

export enum MessageType {
  StateSubmitting,
  SubmitName,
  NameSubmitted,
  UnsubmitName,
  NameUnsubmitted,
  NumNames,
  RequestPlayingState,
  Names,
  GuessName,
  NameGuessed,
  UnguessName,
  NameUnguessed,
  RequestSubmittingState,
}

export type Uuid = string;

export type StateSubmittingMessage = {
  type: MessageType.StateSubmitting;
  content: number;
};

export type SubmitNameMessage = {
  type: MessageType.SubmitName;
  content: string;
};

export type NameSubmittedMessage = {
  type: MessageType.NameSubmitted;
  content: [string, Uuid];
};

export type UnsubmitNameMessage = {
  type: MessageType.UnsubmitName;
  content: Uuid;
};

export type NameUnsubmittedMessage = {
  type: MessageType.NameUnsubmitted;
  content: Uuid;
};

export type NumNamesMessage = {
  type: MessageType.NumNames;
  content: number;
};

export type RequestPlayingStateMessage = {
  type: MessageType.RequestPlayingState;
  content: null;
};

export type NamesMessage = {
  type: MessageType.Names;
  content: [string[], boolean[]];
};

export type GuessNameMessage = {
  type: MessageType.GuessName;
  content: number;
};

export type NameGuessedMessage = {
  type: MessageType.NameGuessed;
  content: number;
};

export type UnguessNameMessage = {
  type: MessageType.UnguessName;
  content: number;
};

export type NameUnguessedMessage = {
  type: MessageType.NameUnguessed;
  content: number;
};

export type RequestSubmittingStateMessage = {
  type: MessageType.RequestSubmittingState;
  content: null;
};

export type Message =
  | StateSubmittingMessage
  | SubmitNameMessage
  | NameSubmittedMessage
  | UnsubmitNameMessage
  | NameUnsubmittedMessage
  | NumNamesMessage
  | RequestPlayingStateMessage
  | NamesMessage
  | GuessNameMessage
  | NameGuessedMessage
  | UnguessNameMessage
  | NameUnguessedMessage
  | RequestSubmittingStateMessage;

function bitfieldToBooleanArray(
  bitfield: Uint8Array,
  arrayLength: number,
): boolean[] {
  const numBytesToProcess = Math.min(
    bitfield.length,
    Math.ceil(arrayLength / 8),
  );
  const array: boolean[] = new Array(numBytesToProcess * 8);

  for (let i = 0; i < numBytesToProcess; i++) {
    const num = bitfield[i];
    const base = i * 8;
    array[base] = !!(num & 0x80);
    array[base + 1] = !!(num & 0x40);
    array[base + 2] = !!(num & 0x20);
    array[base + 3] = !!(num & 0x10);
    array[base + 4] = !!(num & 0x8);
    array[base + 5] = !!(num & 0x4);
    array[base + 6] = !!(num & 0x2);
    array[base + 7] = !!(num & 0x1);
  }

  if (array.length < arrayLength) {
    array.push(...new Array(arrayLength - array.length).fill(false));
  } else {
    array.splice(arrayLength);
  }

  return array;
}

export function decodeMessage(message: ArrayBuffer): Message {
  const view = new DataView(message);
  const type = view.getUint32(0);
  if (view.byteLength === 4) {
    return { type, content: null };
  }

  let content = decode(new Uint8Array(message, 4)) as Message['content'];
  switch (type) {
    case MessageType.Names: {
      const [names, guessesBitfield] = content as unknown as [
        string[],
        Uint8Array,
      ];
      const guesses = bitfieldToBooleanArray(guessesBitfield, names.length);
      content = [names, guesses];
      break;
    }
    case MessageType.NameSubmitted: {
      const [name, id] = content as unknown as [string, Uint8Array];
      content = [name, stringify(id)];
      break;
    }
    case MessageType.UnsubmitName:
    case MessageType.NameUnsubmitted:
      content = stringify(content as unknown as Uint8Array);
      break;
  }

  return { type, content };
}

function booleanArrayToBitfield(array: boolean[]): Uint8Array {
  const bitfield = new Uint8Array(Math.ceil(array.length / 8));
  for (let i = 0; i < array.length; i++) {
    bitfield[Math.floor(i / 8)] |= Number(array[i]) << (7 - (i % 8));
  }
  return bitfield;
}

export function encodeMessage(message: Message): ArrayBuffer {
  let content: Uint8Array | null;
  switch (message.type) {
    case MessageType.Names:
      content = encode([
        message.content[0],
        booleanArrayToBitfield(message.content[1]),
      ]);
      break;
    case MessageType.NameSubmitted:
      content = encode([message.content[0], parse(message.content[1])]);
      break;
    case MessageType.UnsubmitName:
    case MessageType.NameUnsubmitted:
      content = encode(parse(message.content));
      break;
    case MessageType.StateSubmitting:
    case MessageType.SubmitName:
    case MessageType.NumNames:
    case MessageType.GuessName:
    case MessageType.NameGuessed:
    case MessageType.UnguessName:
    case MessageType.NameUnguessed:
      content = encode(message.content);
      break;
    case MessageType.RequestSubmittingState:
    case MessageType.RequestPlayingState:
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
