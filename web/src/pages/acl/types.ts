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
  aliases?: AclAlias[];
};

export type AclCreateContextLoaded = {
  groups: GroupInfo[];
  users: User[];
  devices: StandaloneDevice[];
  networks: Network[];
  aliases: AclAlias[];
  ruleToEdit?: AclRuleInfo;
};

export type AclAlias = {
  id: number;
  name: string;
  destination: string;
  ports: string;
  protocols: AclProtocol[];
};

export type AclAliasPost = Omit<AclAlias, 'id'>;

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
