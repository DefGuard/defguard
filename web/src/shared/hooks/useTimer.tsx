import { useEffect, useRef, useState } from 'react';
import { Subject, timer } from 'rxjs';
import { map, switchMap, take } from 'rxjs/operators';

export interface UseTimerOptions {
  onComplete?: () => void;
}

export const useTimer = (options: UseTimerOptions = {}) => {
  const start$ = useRef(new Subject<number>());
  const onCompleteRef = useRef(options.onComplete);
  const [secondsLeft, setSecondsLeft] = useState(0);

  useEffect(() => {
    onCompleteRef.current = options.onComplete;
  }, [options.onComplete]);

  useEffect(() => {
    const sub = start$.current
      .pipe(
        switchMap((seconds) =>
          timer(0, 1000).pipe(
            map((tick) => seconds - tick),
            take(seconds + 1),
          ),
        ),
      )
      .subscribe({
        next: (value) => setSecondsLeft(value),
        complete: () => onCompleteRef.current?.(),
      });
    return () => {
      sub.unsubscribe();
    };
  }, []);

  return {
    secondsLeft,
    start: (seconds: number) => {
      setSecondsLeft(seconds);
      start$.current.next(seconds);
    },
  };
};
