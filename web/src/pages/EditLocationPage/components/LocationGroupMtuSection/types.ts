/** Client MTU value applied to members of the listed groups. */
export type GroupClientMtu = {
  client_mtu: number;
  group_ids: number[];
};

export const minMtu = 72;
export const maxMtu = 0xffffffff;
