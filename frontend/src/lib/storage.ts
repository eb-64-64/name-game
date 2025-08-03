import { parse, stringify } from 'uuid';

const NAMES_KEY = 'names';

export function getNames(curEpoch: number): [string, Uint8Array][] {
  let json = window.sessionStorage.getItem(NAMES_KEY);
  if (json === null) {
    return [];
  }

  let [storedEpoch, names] = JSON.parse(json) as [number, [string, string][]];
  if (curEpoch === storedEpoch) {
    return names.map(([name, id]) => [name, parse(id)]);
  } else {
    window.sessionStorage.removeItem(NAMES_KEY);
    return [];
  }
}

export function setNames(epoch: number, names: [string, Uint8Array][]) {
  let json = JSON.stringify([
    epoch,
    names.map(([name, id]) => [name, stringify(id)]),
  ]);
  window.sessionStorage.setItem(NAMES_KEY, json);
}

export function clearNames() {
  window.sessionStorage.removeItem(NAMES_KEY);
}
