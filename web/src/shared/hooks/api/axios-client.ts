import axios from 'axios';

const envBaseUrl: string | undefined = import.meta.env.VITE_API_BASE_URL;

const axiosClient = axios.create({
  baseURL: envBaseUrl && String(envBaseUrl).length > 0 ? envBaseUrl : '/api/v1',
});

axiosClient.defaults.headers.common['Content-Type'] = 'application/json';

export default axiosClient;
