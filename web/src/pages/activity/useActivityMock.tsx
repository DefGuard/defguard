import { useEffect, useState } from 'react';

import data from './mock.json';

export type ActivityMock = {
  id: number;
  date: string;
  user: string;
  ip: string;
  event: string;
  module: string;
  device: string;
  details: string;
};

const mockDetails =
  'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam molestie, massa ut ultrices scelerisque, magna dolor efficitur arcu, quis luctus quam nisl at elit. Integer nec egestas est. Vestibulum nec sapien faucibus, ultricies sapien nec, dapibus orci. Sed vitae placerat quam. Nulla facilisi. Nam in suscipit metus.';

export const useActivityMock = () => {
  const [state, setState] = useState<ActivityMock[]>([]);

  useEffect(() => {
    setState(data.map((row, index) => ({ ...row, details: mockDetails, id: index })));
  }, [setState]);

  return state;
};
