import { AclAlias } from '../../../types';
import { ListTagDisplay } from '../shared/types';

export type AclAliasListData = {
  display: {
    ports: ListTagDisplay[];
    destination: ListTagDisplay[];
    protocols: ListTagDisplay[];
    rules: ListTagDisplay[];
  };
} & AclAlias;

export type AclAliasListSelection = Record<string, boolean | undefined>;
