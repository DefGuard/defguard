import create from 'zustand';

import { UseModalStore } from '../../types';
/**
 * Store for modal states.
 * All modals use this store, it controls their visibility and provides extra values.
 */
export const useModalStore = create<UseModalStore>((set, get) => ({
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
  addWebhookModal: {
    visible: false,
  },
  editWebhookModal: {
    visible: false,
    webhook: undefined,
  },
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
  gatewaySetupModal: {
    visible: false,
  },
  userDeviceModal: {
    visible: false,
    currentStep: 0,
    endStep: 1,
    config: undefined,
    deviceName: undefined,
    nextStep: () => {
      const { currentStep, endStep } = get().userDeviceModal;
      if (!(currentStep < endStep)) {
        set((state) => ({
          userDeviceModal: {
            ...state.userDeviceModal,
            currentStep: currentStep + 1,
          },
        }));
      } else {
        if (currentStep === endStep) {
          set((state) => ({
            userDeviceModal: {
              ...state.userDeviceModal,
              currentStep: 0,
              config: undefined,
              deviceName: undefined,
              visible: false,
            },
          }));
        }
      }
    },
  },
  deleteUserDeviceModal: {
    visible: false,
    device: undefined,
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
  setDeleteUserDeviceModal: (newState) =>
    set((oldState) => ({
      deleteUserDeviceModal: { ...oldState.userDeviceModal, ...newState },
    })),
  setUserDeviceModal: (newState) =>
    set((oldState) => ({
      userDeviceModal: { ...oldState.userDeviceModal, ...newState },
    })),
  setAddUserModal: (v) =>
    set((state) => ({ addUserModal: { ...state.addUserModal, ...v } })),
  setAddWebhookModal: (v) =>
    set((state) => ({ addWebhookModal: { ...state.addWebhookModal, ...v } })),
  setEditWebhookModal: (v) =>
    set((state) => ({ editWebhookModal: { ...state.editWebhookModal, ...v } })),
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
  setGatewaySetupModal: (v) =>
    set((state) => ({
      gatewaySetupModal: { ...state.gatewaySetupModal, ...v },
    })),
}));
