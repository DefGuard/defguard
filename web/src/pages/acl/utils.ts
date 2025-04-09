import { SelectOption } from '../../shared/defguard-ui/components/Layout/Select/types';
import { AclRuleInfo } from '../../shared/types';
import { ListTagDisplay } from './AclIndexPage/components/shared/types';
import { AclAliasStatus, AclProtocol, AclStatus } from './types';

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

export const aclAliasStatusToInt = (status: AclAliasStatus): number => {
  switch (status) {
    case AclAliasStatus.APPLIED:
      return 2;
    case AclAliasStatus.MODIFIED:
      return 1;
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
      throw Error(
        `AclStatus conversion from ${statusInt} not possible, returned 'New' instead.`,
      );
  }
};
export const aclAliasStatusFromInt = (statusInt: number): AclAliasStatus => {
  switch (statusInt) {
    case 1:
      return AclAliasStatus.APPLIED;
    case 2:
      return AclAliasStatus.MODIFIED;
    default:
      throw Error(`Unexpected alias status code of ${statusInt}`);
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

export const aclDestinationToListTagDisplay = (destination: string): ListTagDisplay[] =>
  destination
    .split(',')
    .filter((s) => s !== '')
    .map((dest, index) => ({
      key: `destination-${index}`,
      label: dest,
      displayAsTag: false,
    }));

export const aclPortsToListTagDisplay = (ports: string): ListTagDisplay[] =>
  ports
    .split(',')
    .filter((s) => s !== '')
    .map((port, index) => ({
      key: `port-${index}`,
      label: port,
      displayAsTag: false,
    }));

export const aclProtocolsToListTagDisplay = (
  protocols: AclProtocol[],
): ListTagDisplay[] =>
  protocols.map((protocol) => ({
    key: protocol.toString(),
    label: protocolToString(protocol),
    displayAsTag: false,
  }));

export const aclRuleToListTagDisplay = (rules: AclRuleInfo[]): ListTagDisplay[] =>
  rules.map((rule) => ({
    key: rule.id,
    label: rule.name,
    displayAsTag: true,
  }));
