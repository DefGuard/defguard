import { SelectOption } from '../../shared/defguard-ui/components/Layout/Select/types';
import { AclProtocol, AclStatus } from './types';

export const aclStatusToInt = (status: AclStatus): number => {
  switch (status) {
    case AclStatus.NEW:
      return 0;
    case AclStatus.MODIFIED:
      return 1;
    case AclStatus.APPLIED:
      return 2;
    case AclStatus.DELETED:
      return 3;
  }
};

export const aclStatusFromInt = (statusInt: number): AclStatus => {
  switch (statusInt) {
    case 0:
      return AclStatus.NEW;
    case 1:
      return AclStatus.MODIFIED;
    case 2:
      return AclStatus.APPLIED;
    case 3:
      return AclStatus.DELETED;
    default:
      console.error(
        `AclStatus conversion from ${statusInt} not possible, returned 'New' instead.`,
      );
      return AclStatus.NEW;
  }
};

export const protocolToString = (value: AclProtocol): string => {
  switch (value) {
    case AclProtocol.TCP:
      return 'TCP';
    case AclProtocol.UDP:
      return 'UDP';
    case AclProtocol.ICMP:
      return 'ICMP';
  }
};

export const protocolOptions: SelectOption<number>[] = [
  {
    key: AclProtocol.TCP,
    label: 'TCP',
    value: AclProtocol.TCP,
  },
  {
    key: AclProtocol.UDP,
    label: 'UDP',
    value: AclProtocol.UDP,
  },
  {
    key: AclProtocol.ICMP,
    label: 'ICMP',
    value: AclProtocol.ICMP,
  },
];
