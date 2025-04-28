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
  kind: AclAliasKind;
  state: AclAliasStatus;
  destination: string;
  ports: string;
  protocols: AclProtocol[];
  rules: number[];
};

export type AclAliasPost = Omit<AclAlias, 'id' | 'rules' | 'state'>;

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
  EXPIRED = 'Expired',
}

export enum AclAliasStatus {
  APPLIED = AclStatus.APPLIED,
  MODIFIED = AclStatus.MODIFIED,
}

export enum AclKind {
  DESTINATION = 'Destination',
  COMPONENT = 'Component',
}
export enum AclAliasKind {
  DESTINATION = AclKind.DESTINATION,
  COMPONENT = AclKind.COMPONENT,
}
