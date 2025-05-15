import { useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';

export const useOverviewTimeSelection = () => {
  const [searchParams, setSearchParams] = useSearchParams();

  const fromValue = useMemo((): number => {
    const searchValue = searchParams.get('from');
    if (searchValue) {
      const parsed = parseInt(searchValue);
      if (parsed && !isNaN(parsed)) {
        return parsed;
      }
    }
    return 1;
  }, [searchParams]);

  const setTimeSelection = useCallback(
    (value: number) => {
      setSearchParams((perv) => ({ ...perv, from: value }));
    },
    [setSearchParams],
  );

  return { from: fromValue, setTimeSelection };
};
