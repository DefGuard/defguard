import { User } from '../types';

export const getUserFullName = (user: User): string =>
  `${user.first_name} ${user.last_name}`;
