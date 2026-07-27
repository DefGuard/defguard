import { createFileRoute } from '@tanstack/react-router';
import z from 'zod';
import { LoginMainPage } from '../../pages/auth/LoginMain/LoginMainPage';

const searchSchema = z.object({
  authError: z.string().optional(),
});

export const Route = createFileRoute('/auth/login')({
  validateSearch: searchSchema,
  component: LoginMainPage,
});
