/* eslint-disable max-len */
import { MockDevice } from './hooks/useDevicesPage';

export const mockDevices: MockDevice[] = [
  {
    id: 1,
    name: 'Server 1',
    assignedIp: '10.10.10.1',
    description:
      'Lorem ipsum dolor sit amet, consectetur adipiscing elit. Quisque nec varius velit. Integer at ligula non sapien facilisis dictum. Sed vitae turpis vitae urna placerat venenatis.',
    addedBy: 'Alice Johnson',
    addedDate: '2023-01-15T08:30:00Z',
    location: [
      { id: 1, name: 'New York' },
      { id: 2, name: 'Los Angeles' },
    ],
  },
  {
    id: 2,
    name: 'Printer',
    assignedIp: '10.10.10.2',
    description:
      'Suspendisse potenti. Curabitur non libero nec lorem tincidunt tristique. Praesent euismod, nunc at sollicitudin ullamcorper, urna neque vestibulum libero, at malesuada risus massa ut nulla.',
    addedBy: 'Michael Smith',
    addedDate: '2023-03-22T14:45:00Z',
    location: [{ id: 3, name: 'Chicago' }],
  },
  {
    id: 3,
    name: 'Router',
    assignedIp: '10.10.10.3',
    description:
      'Fusce ut urna vitae odio consequat commodo. Nulla facilisi. Aenean tincidunt, sapien nec egestas gravida, ligula urna tincidunt dui, a dictum lacus nunc eget arcu.',
    addedBy: 'Emily Davis',
    addedDate: '2023-06-10T09:20:00Z',
    location: [
      { id: 4, name: 'Houston' },
      { id: 5, name: 'Phoenix' },
    ],
  },
  {
    id: 4,
    name: 'Switch',
    assignedIp: '10.10.10.4',
    description:
      'Maecenas eget metus at metus ultrices lacinia. Donec vel magna vitae urna egestas bibendum. Integer ac nibh non turpis volutpat pulvinar.',
    addedBy: 'Christopher Brown',
    addedDate: '2023-09-05T16:00:00Z',
    location: [{ id: 6, name: 'Philadelphia' }],
  },
  {
    id: 5,
    name: 'Workstation 1',
    assignedIp: '10.10.10.5',
    description:
      'Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae.',
    addedBy: 'Jessica Wilson',
    addedDate: '2023-11-18T11:15:00Z',
    location: [
      { id: 7, name: 'San Antonio' },
      { id: 8, name: 'San Diego' },
    ],
  },
  {
    id: 6,
    name: 'Server 2',
    assignedIp: '10.10.10.6',
    description:
      'Nullam tincidunt, nisl eget bibendum consectetur, sapien justo cursus urna, at ultrices ligula dui non massa. Integer vitae metus a arcu venenatis luctus.',
    addedBy: 'David Martinez',
    addedDate: '2023-02-10T10:00:00Z',
    location: [{ id: 9, name: 'Dallas' }],
  },
  {
    id: 7,
    name: 'Printer 2',
    assignedIp: '10.10.10.7',
    description:
      'Proin eget tortor risus. Vivamus suscipit tortor eget felis porttitor volutpat. Curabitur arcu erat, accumsan id imperdiet et, porttitor at sem.',
    addedBy: 'Sarah Lee',
    addedDate: '2023-04-25T13:30:00Z',
    location: [{ id: 10, name: 'San Jose' }],
  },
  {
    id: 8,
    name: 'Router 2',
    assignedIp: '10.10.10.8',
    description:
      'Praesent sapien massa, convallis a pellentesque nec, egestas non nisi. Nulla porttitor accumsan tincidunt. Nulla quis lorem ut libero malesuada feugiat.',
    addedBy: 'James Anderson',
    addedDate: '2023-05-18T09:45:00Z',
    location: [
      { id: 11, name: 'Austin' },
      { id: 12, name: 'Jacksonville' },
    ],
  },
  {
    id: 9,
    name: 'Switch 2',
    assignedIp: '10.10.10.9',
    description:
      'Curabitur aliquet quam id dui posuere blandit. Donec sollicitudin molestie malesuada. Vivamus magna justo, lacinia eget consectetur sed, convallis at tellus.',
    addedBy: 'Patricia Thomas',
    addedDate: '2023-07-12T15:20:00Z',
    location: [{ id: 13, name: 'Fort Worth' }],
  },
  {
    id: 10,
    name: 'Workstation 2',
    assignedIp: '10.10.10.10',
    description:
      'Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Donec velit neque, auctor sit amet aliquam vel, ullamcorper sit amet ligula.',
    addedBy: 'Robert Taylor',
    addedDate: '2023-08-30T12:10:00Z',
    location: [{ id: 14, name: 'Columbus' }],
  },
  {
    id: 11,
    name: 'Server 3',
    assignedIp: '10.10.10.11',
    description:
      'Quisque velit nisi, pretium ut lacinia in, elementum id enim. Nulla quis lorem ut libero malesuada feugiat. Donec sollicitudin molestie malesuada.',
    addedBy: 'Linda Moore',
    addedDate: '2023-01-22T07:50:00Z',
    location: [
      { id: 15, name: 'Charlotte' },
      { id: 16, name: 'San Francisco' },
    ],
  },
  {
    id: 12,
    name: 'Printer 3',
    assignedIp: '10.10.10.12',
    description:
      'Mauris blandit aliquet elit, eget tincidunt nibh pulvinar a. Nulla porttitor accumsan tincidunt. Donec rutrum congue leo eget malesuada.',
    addedBy: 'Barbara Jackson',
    addedDate: '2023-03-14T10:30:00Z',
    location: [{ id: 17, name: 'Indianapolis' }],
  },
  {
    id: 13,
    name: 'Router 3',
    assignedIp: '10.10.10.13',
    description:
      'Sed porttitor lectus nibh. Curabitur arcu erat, accumsan id imperdiet et, porttitor at sem. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae.',
    addedBy: 'Thomas Harris',
    addedDate: '2023-04-27T14:00:00Z',
    location: [{ id: 18, name: 'Seattle' }],
  },
  {
    id: 14,
    name: 'Switch 3',
    assignedIp: '10.10.10.14',
    description:
      'Pellentesque in ipsum id orci porta dapibus. Donec rutrum congue leo eget malesuada. Nulla quis lorem ut libero malesuada feugiat.',
    addedBy: 'Daniel Clark',
    addedDate: '2023-06-05T11:25:00Z',
    location: [
      { id: 19, name: 'Denver' },
      { id: 20, name: 'Washington' },
    ],
  },
  {
    id: 15,
    name: 'Workstation 3',
    assignedIp: '10.10.10.15',
    description:
      'Vivamus suscipit tortor eget felis porttitor volutpat. Nulla quis lorem ut libero malesuada feugiat. Nulla porttitor accumsan tincidunt.',
    addedBy: 'Susan Lewis',
    addedDate: '2023-07-19T09:15:00Z',
    location: [{ id: 21, name: 'Boston' }],
  },
  {
    id: 16,
    name: 'Server 4',
    assignedIp: '10.10.10.16',
    description:
      'Proin eget tortor risus. Curabitur arcu erat, accumsan id imperdiet et, porttitor at sem. Nulla porttitor accumsan tincidunt.',
    addedBy: 'Kevin Walker',
    addedDate: '2023-09-23T16:40:00Z',
    location: [
      { id: 22, name: 'El Paso' },
      { id: 23, name: 'Nashville' },
    ],
  },
  {
    id: 17,
    name: 'Printer 4',
    assignedIp: '10.10.10.17',
    description:
      'Donec rutrum congue leo eget malesuada. Donec sollicitudin molestie malesuada. Nulla quis lorem ut libero malesuada feugiat.',
    addedBy: 'Karen Hall',
    addedDate: '2023-10-12T13:55:00Z',
    location: [{ id: 24, name: 'Detroit' }],
  },
  {
    id: 18,
    name: 'Router 4',
    assignedIp: '10.10.10.18',
    description:
      'Mauris blandit aliquet elit, eget tincidunt nibh pulvinar a. Nulla porttitor accumsan tincidunt. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae.',
    addedBy: 'Brian Allen',
    addedDate: '2023-11-05T08:20:00Z',
    location: [{ id: 25, name: 'Oklahoma City' }],
  },
  {
    id: 19,
    name: 'Switch 4',
    assignedIp: '10.10.10.19',
    description:
      'Curabitur aliquet quam id dui posuere blandit. Vestibulum ante ipsum primis in faucibus orci luctus et ultrices posuere cubilia curae; Donec rutrum congue leo eget malesuada.',
    addedBy: 'Nancy Young',
    addedDate: '2023-12-01T17:35:00Z',
    location: [
      { id: 26, name: 'Portland' },
      { id: 27, name: 'Las Vegas' },
    ],
  },
  {
    id: 20,
    name: 'Workstation 4',
    assignedIp: '10.10.10.20',
    description:
      'Nulla quis lorem ut libero malesuada feugiat. Proin eget tortor risus. Donec sollicitudin molestie malesuada.',
    addedBy: 'George Hernandez',
    addedDate: '2023-02-28T12:50:00Z',
    location: [{ id: 28, name: 'Memphis' }],
  },
  {
    id: 21,
    name: 'Server 5',
    assignedIp: '10.10.10.21',
    description:
      'Donec rutrum congue leo eget malesuada. Vivamus magna justo, lacinia eget consectetur sed, convallis at tellus. Donec rutrum congue leo eget malesuada.',
    addedBy: 'Donna King',
    addedDate: '2023-03-10T10:10:00Z',
    location: [{ id: 29, name: 'Louisville' }],
  },
  {
    id: 22,
    name: 'Printer 5',
    assignedIp: '10.10.10.22',
    description:
      'Curabitur arcu erat, accumsan id imperdiet et, porttitor at sem. Nulla porttitor accumsan tincidunt. Pellentesque in ipsum id orci porta dapibus.',
    addedBy: 'Steven Wright',
    addedDate: '2023-04-18T14:25:00Z',
    location: [
      { id: 30, name: 'Baltimore' },
      { id: 31, name: 'Milwaukee' },
    ],
  },
  {
    id: 23,
    name: 'Router 5',
    assignedIp: '10.10.10.23',
    description:
      'Pellentesque in ipsum id orci porta dapibus. Donec rutrum congue leo eget malesuada. Nulla quis lorem ut libero malesuada feugiat.',
    addedBy: 'Laura Lopez',
    addedDate: '2023-05-30T09:05:00Z',
    location: [{ id: 32, name: 'Albuquerque' }],
  },
  {
    id: 24,
    name: 'Switch 5',
    assignedIp: '10.10.10.24',
    description:
      'Vivamus magna justo, lacinia eget consectetur sed, convallis at tellus. Donec sollicitudin molestie malesuada. Curabitur non nulla sit amet nisl tempus convallis quis ac lectus.',
    addedBy: 'Paul Hill',
    addedDate: '2023-06-20T11:45:00Z',
    location: [{ id: 33, name: 'Tucson' }],
  },
  {
    id: 25,
    name: 'Workstation 5',
    assignedIp: '10.10.10.25',
    description:
      'Proin eget tortor risus. Donec sollicitudin molestie malesuada. Nulla porttitor accumsan tincidunt.',
    addedBy: 'Jennifer Scott',
    addedDate: '2023-07-08T16:30:00Z',
    location: [{ id: 34, name: 'Fresno' }],
  },
  {
    id: 26,
    name: 'Server 6',
    assignedIp: '10.10.10.26',
    description:
      'Mauris blandit aliquet elit, eget tincidunt nibh pulvinar a. Nulla porttitor accumsan tincidunt. Nulla quis lorem ut libero malesuada feugiat.',
    addedBy: 'Mark Green',
    addedDate: '2023-08-14T13:00:00Z',
    location: [
      { id: 35, name: 'Sacramento' },
      { id: 36, name: 'Kansas City' },
    ],
  },
  {
    id: 27,
    name: 'Printer 6',
    assignedIp: '10.10.10.27',
    description:
      'Donec rutrum congue leo eget malesuada. Curabitur arcu erat, accumsan id imperdiet et, porttitor at sem. Nulla porttitor accumsan tincidunt.',
    addedBy: 'Elizabeth Adams',
    addedDate: '2023-09-19T10:40:00Z',
    location: [{ id: 37, name: 'Long Beach' }],
  },
  {
    id: 28,
    name: 'Router 6',
    assignedIp: '10.10.10.28',
    description:
      'Pellentesque in ipsum id orci porta dapibus. Donec rutrum congue leo eget malesuada. Nulla quis lorem ut libero malesuada feugiat.',
    addedBy: 'Charles Baker',
    addedDate: '2023-10-25T15:15:00Z',
    location: [{ id: 38, name: 'Mesa' }],
  },
  {
    id: 29,
    name: 'Switch 6',
    assignedIp: '10.10.10.29',
    description:
      'Curabitur arcu erat, accumsan id imperdiet et, porttitor at sem. Nulla porttitor accumsan tincidunt. Donec rutrum congue leo eget malesuada.',
    addedBy: 'Barbara Gonzalez',
    addedDate: '2023-11-30T12:05:00Z',
    location: [
      { id: 39, name: 'Atlanta' },
      { id: 40, name: 'Omaha' },
    ],
  },
  {
    id: 30,
    name: 'Workstation 6',
    assignedIp: '10.10.10.30',
    description:
      'Vivamus suscipit tortor eget felis porttitor volutpat. Nulla quis lorem ut libero malesuada feugiat. Donec sollicitudin molestie malesuada.',
    addedBy: 'Richard Nelson',
    addedDate: '2023-12-12T09:50:00Z',
    location: [{ id: 41, name: 'Colorado Springs' }],
  },
];
