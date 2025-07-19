import { decode, encode } from '@msgpack/msgpack';

export enum MessageType {
  SubmissionTime,
  Submission,
  NumSubmissions,
  PlayTime,
  SubmissionList,
}

export type SubmissionTimeMessage = {
  type: MessageType.SubmissionTime;
  content: null;
};

export type SubmissionMessage = {
  type: MessageType.Submission;
  content: string;
};

export type NumSubmissionsMessage = {
  type: MessageType.NumSubmissions;
  content: number;
};

export type PlayTimeMessage = {
  type: MessageType.PlayTime;
  content: null;
};

export type SubmissionListMessage = {
  type: MessageType.SubmissionList;
  content: string[];
};

export type Message =
  | SubmissionTimeMessage
  | SubmissionMessage
  | NumSubmissionsMessage
  | PlayTimeMessage
  | SubmissionListMessage;

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
