import { Variants } from 'framer-motion';

export const tableBodyVariants: Variants = {
  hidden: {
    opacity: 0,
  },
  idle: {
    opacity: 1,
  },
};

export const tableRowVariants: Variants = {
  idle: () => ({
    boxShadow: '5px 10px 20px rgba(0, 0, 0, 0)',
  }),
  hover: {
    boxShadow: '5px 10px 20px rgba(0, 0, 0, 0.1)',
  },
};

export const rowIconVariants: Variants = {
  idle: {
    opacity: 0,
  },
  hover: {
    opacity: 1,
  },
};

export const standardVariants: Variants = {
  hidden: {
    opacity: 0,
  },
  show: {
    opacity: 1,
  },
};
