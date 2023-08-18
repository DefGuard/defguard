import type { SVGProps } from 'react';
const SvgIconDownload = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    fill="none"
    viewBox="0 0 22 22"
    {...props}
  >
    <path
      fill="#899CA8"
      d="M18 13v4a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-4a1 1 0 1 1 2 0v3h10v-3a1 1 0 1 1 2 0Z"
    />
    <path
      fill="#899CA8"
      d="M8.871 7.707a1 1 0 1 0-1.414 1.414l2.828 2.829a.998.998 0 0 0 .79.29c.285.025.579-.071.797-.29l2.829-2.828a1 1 0 0 0-1.414-1.414L12 8.995V5a1 1 0 1 0-2 0v3.836L8.871 7.707Z"
    />
  </svg>
);
export default SvgIconDownload;
