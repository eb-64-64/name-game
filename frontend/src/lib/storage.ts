import type { Uuid } from './messages';

const NAMES_KEY = 'names';

export function getNames(curEpoch: number): [string, Uuid][] {
  const json = window.sessionStorage.getItem(NAMES_KEY);
  if (json === null) {
    return [];
  }

  const [storedEpoch, names] = JSON.parse(json) as [number, [string, Uuid][]];
  if (curEpoch === storedEpoch) {
    return names;
  } else {
    window.sessionStorage.removeItem(NAMES_KEY);
    return [];
  }
}

export function setNames(epoch: number, names: [string, Uuid][]) {
  const json = JSON.stringify([epoch, names]);
  window.sessionStorage.setItem(NAMES_KEY, json);
}

export function clearNames() {
  window.sessionStorage.removeItem(NAMES_KEY);
}
