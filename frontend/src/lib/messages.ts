import { decode, encode } from '@msgpack/msgpack';

export enum MessageType {
  Submitting,
  Name,
  NumNames,
  NotSubmitting,
  Names,
}

export type SubmittingMessage = {
  type: MessageType.Submitting;
  content: null;
};

export type NameMessage = {
  type: MessageType.Name;
  content: string;
};

export type NumNamesMessage = {
  type: MessageType.NumNames;
  content: number;
};

export type NotSubmittingMessage = {
  type: MessageType.NotSubmitting;
  content: null;
};

export type NamesMessage = {
  type: MessageType.Names;
  content: string[];
};

export type Message =
  | SubmittingMessage
  | NameMessage
  | NumNamesMessage
  | NotSubmittingMessage
  | NamesMessage;

export function parseMessage(message: ArrayBuffer): Message {
  const view = new DataView(message);
  const type = view.getUint32(0);
  const len = view.getUint32(4);
  if (len == 0) {
    return { type, content: null };
  }

  const content = decode(new Uint8Array(message, 8)) as Message['content'];
  return { type, content };
}

export function encodeMessage(message: Message): ArrayBuffer {
  const content = message.content !== null ? encode(message.content) : null;
  const len = content?.byteLength ?? 0;

  const encoded = new ArrayBuffer(8 + len);
  const view = new DataView(encoded);
  view.setUint32(0, message.type);
  view.setUint32(4, len);

  if (content !== null) {
    new Uint8Array(encoded, 8).set(content);
  }

  return encoded;
}
