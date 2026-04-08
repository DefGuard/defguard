import { create } from 'zustand';

type Store = { isOpen: boolean };

export const useVideoTutorialsModal = create<Store>(() => ({ isOpen: false }));
