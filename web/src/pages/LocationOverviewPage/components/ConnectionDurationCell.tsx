import type { CSSProperties } from 'react';
import { useCallback, useEffect, useState } from 'react';
import { timer } from 'rxjs';
import { TableCell } from '../../../shared/defguard-ui/components/table/TableCell/TableCell';
import { formatConnectionTime } from '../../../shared/utils/formatConnectionTime';

type Props = {
  connectedAt: string;
  style?: CSSProperties;
};

export const ConnectionDurationCell = ({ connectedAt, style }: Props) => {
  const [displayedTime, setDisplayedTime] = useState<string | undefined>();

  const updateConnectionTime = useCallback(() => {
    if (connectedAt) {
      setDisplayedTime(formatConnectionTime(connectedAt));
    }
    return '';
  }, [connectedAt]);

  useEffect(() => {
    const interval = 60 * 1000;
    const sub = timer(0, interval).subscribe(() => {
      updateConnectionTime();
    });

    return () => {
      sub.unsubscribe();
    };
  }, [updateConnectionTime]);

  return (
    <TableCell style={style}>
      <span>{displayedTime}</span>
    </TableCell>
  );
};
