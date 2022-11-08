import { ReactNode, useEffect, useState } from 'react';

interface Props {
  delay: number;
  fallback: ReactNode | null;
  children?: ReactNode;
}

export const DelayRender = ({ delay, fallback, children }: Props) => {
  const [isShown, setIsShown] = useState(false);

  useEffect(() => {
    setTimeout(() => {
      setIsShown(true);
    }, delay);
  }, [delay]);

  return (
    <>
      {isShown && children ? children : null}
      {!isShown && fallback ? fallback : null}
    </>
  );
};
