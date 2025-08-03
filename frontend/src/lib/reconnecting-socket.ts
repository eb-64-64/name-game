import { decodeMessage, encodeMessage, type Message } from './messages';

export type OpenHandler = (() => void) | null;
export type MessageHandler = ((message: Message) => void) | null;
export type CloseHandler = (() => void) | null;

export class ReconnectingSocket {
  private ws: WebSocket | null = null;
  private url: string | URL;

  private _onOpen: OpenHandler = null;
  private _onMessage: MessageHandler = null;
  private _onClose: CloseHandler = null;

  private attempt = 0;
  private reconnectTimeout: number | null = null;

  private MAX_ATTEMPTS = 10;

  constructor(url: string | URL) {
    this.url = url;
    this.connect();
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'hidden') {
        if (this.ws !== null) {
          this.ws.close();
        }
        if (this.reconnectTimeout !== null) {
          clearTimeout(this.reconnectTimeout);
          this.reconnectTimeout = null;
        }
      } else {
        this.connect();
      }
    });
  }

  public get connected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }

  public set onOpen(handler: OpenHandler) {
    this._onOpen = handler;
  }

  public set onMessage(handler: MessageHandler) {
    this._onMessage = handler;
  }

  public set onClose(handler: CloseHandler) {
    this._onClose = handler;
  }

  public send(message: Message) {
    if (this.connected) {
      this.ws!.send(encodeMessage(message));
    } else {
      console.warn('WebSocket not connected', message);
    }
  }

  private connect() {
    this.ws = new WebSocket(this.url);
    this.ws.binaryType = 'arraybuffer';
    this.ws.addEventListener('open', () => {
      this._onOpen?.();
      this.attempt = 0;
    });
    this.ws.addEventListener('message', (event) => {
      const message = decodeMessage(event.data);
      this._onMessage?.(message);
    });
    this.ws.addEventListener('close', () => {
      this.ws = null;
      this._onClose?.();
      if (document.visibilityState === 'visible') {
        console.warn('WebSocket closed. Reconnecting...');
        this.reconnect();
      }
    });
    this.ws.addEventListener('error', (err) => {
      console.error('WebSocket error:', err);
      this.ws!.close();
    });
  }

  public close() {
    if (this.connected) {
      this.ws!.close();
    }
  }

  private reconnect() {
    if (this.attempt >= this.MAX_ATTEMPTS) {
      console.error('Max reconnection attempts reached');
      return;
    }

    if (this.reconnectTimeout === null) {
      const delay = this.getBackoffDelay();
      console.log(`Reconnecting in ${delay} ms...`);
      this.reconnectTimeout = setTimeout(() => {
        this.reconnectTimeout = null;
        this.attempt += 1;
        this.connect();
      }, delay);
    }
  }

  private getBackoffDelay() {
    const BASE = 50;
    const MAX = 3000;

    const jitter = Math.random() * 100;
    return Math.min(BASE * 2 ** this.attempt + jitter, MAX);
  }
}
