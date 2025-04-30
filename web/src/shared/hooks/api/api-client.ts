import { buildApi } from './api';
import axiosClient from './axios-client';

const apiEndpoints = buildApi(axiosClient);

export default apiEndpoints;
