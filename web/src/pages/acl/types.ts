import { GroupInfo, Network, StandaloneDevice, User } from '../../shared/types';

export type ACLRule = {
  id?: number;
  name: string;
  all_networks: boolean;
  networks: Array<number>;
  allow_all_users: boolean;
  deny_all_users: boolean;
  allowed_users: Array<number>;
  denied_users: Array<number>;
  allowed_groups: Array<number>;
  denied_groups: Array<number>;
  destination: Array<number>;
  aliases: Array<number>;
  ports: Array<number>;
  expires?: string;
};

export type AclCreateContext = {
  groups?: GroupInfo[];
  users?: User[];
  devices?: StandaloneDevice[];
  networks?: Network[];
};

export type AclCreateContextLoaded = {
  groups: GroupInfo[];
  users: User[];
  devices: StandaloneDevice[];
  networks: Network[];
};

export enum AclProtocol {
  TCP = 6,
  UDP = 17,
  ICMP = 1,
}
