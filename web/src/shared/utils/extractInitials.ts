export const extractInitials = (val: string): string => {
  const sp = val.split(' ');
  try {
    const res = `${sp[0][0].toUpperCase()}${
      sp[1] ? sp[1][0].toUpperCase() : ''
    }`;
    return res;
  } catch (error) {
    console.error(error);
    return '';
  }
};
