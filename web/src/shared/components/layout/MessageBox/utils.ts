const storageKey = 'message-box-visibility';

type Store = {
  [key: string]: boolean;
};

// checks if message box should be visible
export const readMessageBoxVisibility = (messageBoxId: string): boolean => {
  const storeString = localStorage.getItem(storageKey);
  if (storeString) {
    const store = JSON.parse(storeString) as Store;
    try {
      const visibility = store[messageBoxId];
      return visibility;
    } catch {
      return true;
    }
  }
  return true;
};

export const writeMessageBoxVisibility = (messageBoxId: string) => {
  const storeString = localStorage.getItem(storageKey);
  if (storeString) {
    const store = JSON.parse(storeString) as Store;
    store[messageBoxId] = false;
    localStorage.setItem(storageKey, JSON.stringify(store));
  } else {
    const store: Store = {};
    store[messageBoxId] = false;
    localStorage.setItem(storageKey, JSON.stringify(store));
  }
};
