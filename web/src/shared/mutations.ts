export enum MutationKeys {
  LOG_IN = 'LOG_IN',
  REGISTER_SECURITY_KEY_START = 'REGISTER_SECURITY_KEY_START',
  REGISTER_SECURITY_KEY_FINISH = 'REGISTER_SECURITY_KEY_FINISH',
  CREATE_WORKER_JOB = 'CREATE_WORKER_JOB',
  CHANGE_PASSWORD = 'CHANGE_PASSWORD',
  SET_WALLET = 'SET_WALLET',
  DELETE_WALLET = 'DELETE_WALLET',
  WALLET_CHALLENGE = 'WALLET_CHALLENGE',
  ADD_USER_TO_GROUP = 'ADD_USER_TO_GROUP',
  ADD_DEVICE = 'ADD_DEVICE',
  REMOVE_USER_FROM_GROUP = 'REMOVE_USER_FROM_GROUP',
  CHANGE_USER_PASSWORD = 'CHANGE_USER_PASSWORD',
  DELETE_WORKER = 'DELETE_WORKER',
  OAUTH_CONSENT = 'OAUTH_CONSENT',
  DELETE_WEBHOOK = 'DELETE_WEBHOOK',
  CHANGE_WEBHOOK_STATE = 'CHANGE_WEBHOOK_STATE',
  EDIT_WEBHOOK = 'EDIT_WEBHOOK',
  CHANGE_CLIENT_STATE = 'CHANGE_CLIENT_STATE',
  REMOVE_USER_CLIENT = 'REMOVE_USER_CLIENT',
  EDIT_USER_DEVICE = 'EDIT_USER_DEVICE',
  DELETE_USER_DEVICE = 'DELETE_USER_DEVICE',
  EDIT_USER = 'EDIT_USER',
  ENABLE_MFA = 'ENABLE_MFA',
  DISABLE_MFA = 'DISABLE_MFA',
  ENABLE_TOTP_INIT = 'ENABLE_TOTP_INIT',
  ENABLE_TOTP_FINISH = 'ENABLE_TOTP_FINISH',
  DISABLE_TOTP = 'DISABLE_TOTP',
  VERIFY_TOTP = 'VERIFY_TOTP',
}
