import { createWithEqualityFn } from 'zustand/traditional';

import { UseModalStore } from '../../types';

/**
 * This approach is deprecated, please use separate stores for each modal to keep things clean
 */
export const useModalStore = createWithEqualityFn<UseModalStore>(
  (set) => ({
    openIdClientModal: {
      client: undefined,
      viewMode: false,
      visible: false,
    },
    setOpenIdClientModal: (newState) =>
      set((oldState) => ({
        openIdClientModal: { ...oldState.openIdClientModal, ...newState },
      })),
    addWalletModal: {
      visible: false,
    },
    recoveryCodesModal: {
      visible: false,
      codes: undefined,
    },
    connectWalletModal: {
      visible: false,
      onConnect: undefined,
    },
    registerTOTP: {
      visible: false,
    },
    provisionKeyModal: {
      visible: false,
      user: undefined,
    },
    deleteUserModal: {
      visible: false,
      user: undefined,
    },
    changePasswordModal: {
      visible: false,
      user: undefined,
    },
    changeWalletModal: {
      visible: false,
      user: undefined,
    },
    keyDetailModal: {
      visible: false,
    },
    keyDeleteModal: {
      visible: false,
    },
    addUserModal: {
      visible: false,
    },
    webhookModal: {
      visible: false,
      webhook: undefined,
    },
    setWebhookModal: (newState) =>
      set((oldState) => ({
        webhookModal: { ...oldState.webhookModal, ...newState },
      })),
    deleteOpenidClientModal: {
      visible: false,
      client: undefined,
      onSuccess: undefined,
    },
    enableOpenidClientModal: {
      visible: false,
      client: undefined,
      onSuccess: undefined,
    },
    addOpenidClientModal: {
      visible: false,
    },
    manageWebAuthNKeysModal: {
      visible: false,
    },
    addSecurityKeyModal: {
      visible: false,
    },
    licenseModal: {
      visible: false,
    },
    setState: (newState) => set((oldState) => ({ ...oldState, ...newState })),
    setRecoveryCodesModal: (newState) =>
      set((oldState) => ({
        recoveryCodesModal: { ...oldState.recoveryCodesModal, ...newState },
      })),
    setAddUserModal: (v) =>
      set((state) => ({ addUserModal: { ...state.addUserModal, ...v } })),
    setKeyDeleteModal: (v) =>
      set((state) => ({ keyDeleteModal: { ...state.keyDeleteModal, ...v } })),
    setKeyDetailModal: (v) =>
      set((state) => ({ keyDetailModal: { ...state.keyDetailModal, ...v } })),
    setChangePasswordModal: (data) =>
      set((state) => ({
        changePasswordModal: { ...state.changePasswordModal, ...data },
      })),
    setChangeWalletModal: (data) =>
      set((state) => ({
        changeWalletModal: { ...state.changeWalletModal, ...data },
      })),
    setDeleteUserModal: (data) =>
      set((state) => ({
        deleteUserModal: { ...state.deleteUserModal, ...data },
      })),
    setProvisionKeyModal: (data) =>
      set((state) => ({
        provisionKeyModal: { ...state.provisionKeyModal, ...data },
      })),
    setAddOpenidClientModal: (v) =>
      set((state) => ({
        addOpenidClientModal: { ...state.addOpenidClientModal, ...v },
      })),
    setLicenseModal: (v) =>
      set((state) => ({
        licenseModal: { ...state.licenseModal, ...v },
      })),
    setDeleteOpenidClientModal: (data) =>
      set((state) => ({
        deleteOpenidClientModal: { ...state.deleteOpenidClientModal, ...data },
      })),
    setEnableOpenidClientModal: (data) =>
      set((state) => ({
        enableOpenidClientModal: { ...state.enableOpenidClientModal, ...data },
      })),
  }),
  Object.is,
);
