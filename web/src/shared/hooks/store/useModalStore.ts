import { createWithEqualityFn } from 'zustand/traditional';

import { UseModalStore } from '../../types';

/**
 * This approach is deprecated, please use separate stores for each modal to keep things clean
 */
export const useModalStore = createWithEqualityFn<UseModalStore>(
  (set) => ({
    // DO NOT EXTEND THIS STORE
    openIdClientModal: {
      client: undefined,
      viewMode: false,
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    setOpenIdClientModal: (newState) =>
      set((oldState) => ({
        openIdClientModal: { ...oldState.openIdClientModal, ...newState },
      })),
    // DO NOT EXTEND THIS STORE
    addWalletModal: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    recoveryCodesModal: {
      visible: false,
      codes: undefined,
    },
    // DO NOT EXTEND THIS STORE
    connectWalletModal: {
      visible: false,
      onConnect: undefined,
    },
    // DO NOT EXTEND THIS STORE
    registerTOTP: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    provisionKeyModal: {
      visible: false,
      user: undefined,
    },
    // DO NOT EXTEND THIS STORE
    deleteUserModal: {
      visible: false,
      user: undefined,
    },
    // DO NOT EXTEND THIS STORE
    toggleUserModal: {
      visible: false,
      user: undefined,
    },
    // DO NOT EXTEND THIS STORE
    changePasswordModal: {
      visible: false,
      user: undefined,
    },
    // DO NOT EXTEND THIS STORE
    changeWalletModal: {
      visible: false,
      user: undefined,
    },
    // DO NOT EXTEND THIS STORE
    keyDetailModal: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    keyDeleteModal: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    webhookModal: {
      visible: false,
      webhook: undefined,
    },
    // DO NOT EXTEND THIS STORE
    setWebhookModal: (newState) =>
      set((oldState) => ({
        webhookModal: { ...oldState.webhookModal, ...newState },
      })),
    // DO NOT EXTEND THIS STORE
    deleteOpenidClientModal: {
      visible: false,
      client: undefined,
      onSuccess: undefined,
    },
    // DO NOT EXTEND THIS STORE
    enableOpenidClientModal: {
      visible: false,
      client: undefined,
      onSuccess: undefined,
    },
    // DO NOT EXTEND THIS STORE
    addOpenidClientModal: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    manageWebAuthNKeysModal: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    addSecurityKeyModal: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    licenseModal: {
      visible: false,
    },
    // DO NOT EXTEND THIS STORE
    setState: (newState) => set((oldState) => ({ ...oldState, ...newState })),
    // DO NOT EXTEND THIS STORE
    setRecoveryCodesModal: (newState) =>
      set((oldState) => ({
        recoveryCodesModal: { ...oldState.recoveryCodesModal, ...newState },
      })),
    // DO NOT EXTEND THIS STORE
    setKeyDeleteModal: (v) =>
      set((state) => ({ keyDeleteModal: { ...state.keyDeleteModal, ...v } })),
    // DO NOT EXTEND THIS STORE
    setKeyDetailModal: (v) =>
      set((state) => ({ keyDetailModal: { ...state.keyDetailModal, ...v } })),
    // DO NOT EXTEND THIS STORE
    setChangePasswordModal: (data) =>
      set((state) => ({
        changePasswordModal: { ...state.changePasswordModal, ...data },
      })),
    // DO NOT EXTEND THIS STORE
    setChangeWalletModal: (data) =>
      set((state) => ({
        changeWalletModal: { ...state.changeWalletModal, ...data },
      })),
    // DO NOT EXTEND THIS STORE
    setDeleteUserModal: (data) =>
      set((state) => ({
        deleteUserModal: { ...state.deleteUserModal, ...data },
      })),
    // DO NOT EXTEND THIS STORE
    setToggleUserModal: (data) =>
      set((state) => ({
        toggleUserModal: { ...state.toggleUserModal, ...data },
      })),
    // DO NOT EXTEND THIS STORE
    setProvisionKeyModal: (data) =>
      set((state) => ({
        provisionKeyModal: { ...state.provisionKeyModal, ...data },
      })),
    // DO NOT EXTEND THIS STORE
    setAddOpenidClientModal: (v) =>
      set((state) => ({
        addOpenidClientModal: { ...state.addOpenidClientModal, ...v },
      })),
    // DO NOT EXTEND THIS STORE
    setDeleteOpenidClientModal: (data) =>
      set((state) => ({
        deleteOpenidClientModal: { ...state.deleteOpenidClientModal, ...data },
      })),
    // DO NOT EXTEND THIS STORE
    setEnableOpenidClientModal: (data) =>
      set((state) => ({
        enableOpenidClientModal: { ...state.enableOpenidClientModal, ...data },
      })),
  }),
  Object.is,
);
