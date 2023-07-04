import { Story } from '@ladle/react';
import { useMemo } from 'react';

import { User, UserMFAMethod } from '../../../types';
import { ListHeader, ListSortDirection, VirtualizedList } from './VirtualizedList';

const headers: ListHeader[] = [
  {
    text: 'test 1',
    active: false,
    sortDirection: ListSortDirection.DESC,
    key: '1',
  },
  {
    text: 'test 02',
    active: true,
    sortDirection: ListSortDirection.DESC,
    key: '2',
  },
  {
    text: 'test 03',
    active: true,
    sortDirection: ListSortDirection.ASC,
    key: '3',
  },
];

export const VirtualizedListStory: Story = () => {
  const data = useMemo(() => mockData(), []);
  return (
    <VirtualizedList
      data={data}
      rowSize={60}
      padding={{
        left: 20,
        right: 20,
      }}
      headerPadding={{
        left: 20,
        right: 20,
      }}
      headers={headers}
    />
  );
};

const mockData = (): User[] => {
  const res: User[] = [];
  for (let i = 0; i < 1000; i++) {
    res.push({
      id: i,
      username: `test${i}`,
      first_name: `Test ${i}`,
      last_name: `Test ${i}`,
      phone: '123456789',
      email: 'test@test.com',
      authorized_apps: [],
      mfa_method: UserMFAMethod.NONE,
      mfa_enabled: false,
      totp_enabled: false,
      groups: [],
    });
  }
  return res;
};
