import { AclAlias } from '../../../types';
import { ListCellTag } from '../shared/types';

export type AclAliasListData = {
  display: {
    ports: ListCellTag[];
    destination: ListCellTag[];
    protocols: ListCellTag[];
    rules: ListCellTag[];
  };
} & AclAlias;

export type AclAliasListSelection = Record<string, boolean | undefined>;
