import {
  AclRuleInfo,
  GroupInfo,
  Network,
  StandaloneDevice,
  User,
} from '../../shared/types';

export type AclCreateContext = {
  groups?: GroupInfo[];
  users?: User[];
  devices?: StandaloneDevice[];
  networks?: Network[];
  editRule?: AclRuleInfo;
};

export type AclCreateContextLoaded = {
  groups: GroupInfo[];
  users: User[];
  devices: StandaloneDevice[];
  networks: Network[];
  ruleToEdit?: AclRuleInfo;
};

export enum AclProtocol {
  TCP = 6,
  UDP = 17,
  ICMP = 1,
}

export enum AclStatus {
  NEW = 'New',
  APPLIED = 'Applied',
  MODIFIED = 'Modified',
  DELETED = 'Deleted',
}
