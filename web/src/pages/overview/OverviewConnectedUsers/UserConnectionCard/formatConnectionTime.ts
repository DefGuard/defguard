import dayjs from 'dayjs';
import humanizeDuration from 'humanize-duration';

const short = humanizeDuration.humanizer({
  language: 'short',
  languages: {
    short: {
      y: () => 'y',
      mo: () => 'mo',
      w: () => 'w',
      d: () => 'd',
      h: () => 'h',
      m: () => 'm',
      s: () => 's',
      ms: () => 'ms',
    },
  },
});

export const formatConnectionTime = (connectedAt: string): string => {
  const day = dayjs.utc(connectedAt);
  const diff = dayjs().utc().diff(day, 'ms');

  const res = short(diff, {
    largest: 2,
    round: true,
    language: 'short',
    units: ['y', 'mo', 'w', 'd', 'h', 'm', 's'],
  })
    .replaceAll(' ', '')
    .replaceAll(',', ' ');
  return res;
};
