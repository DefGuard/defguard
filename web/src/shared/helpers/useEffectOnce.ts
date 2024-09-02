import { useEffect, useRef } from 'react';

/** 
Under normal circumstances, useEffect should run only once when passed an empty dependency array.
However, in dev mode with react strict mode enabled, everything is rendered twice for debugging purposes.
This also causes useEffect to run twice, which is not always desirable.
This custom hook ensures that the effect runs only once in dev mode as well.
*/
export default function useEffectOnce(fn: () => void) {
  const isMounted = useRef(false);
  useEffect(() => {
    if (isMounted.current) {
      return;
    }

    fn();
    isMounted.current = true;
  }, [fn]);
}
