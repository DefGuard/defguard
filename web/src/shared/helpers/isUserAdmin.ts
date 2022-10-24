import { isUndefined } from 'lodash-es';

import { User } from '../types';

const adminGroupName = 'admin';

export const isUserAdmin = (user: User): boolean => {
  if (!isUndefined(user.groups.find((group) => group === adminGroupName))) {
    return true;
  }
  return false;
};
