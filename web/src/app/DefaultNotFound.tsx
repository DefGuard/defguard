import { Navigate } from '@tanstack/react-router';

export const DefaultNotFound = () => {
  return <Navigate to="/404" />;
};
