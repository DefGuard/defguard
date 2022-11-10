import create from "zustand";
import { ToastType } from "../../components/layout/Toast/Toast";

export interface Toast {
    id: number;
    message: string;
    type: ToastType;
    subMessage?: string;
}
export interface ToastStore {
    toasts: Toast[];
}

const useToastsStore = create<ToastStore>((set,get) => ({
    toasts: [],
}));