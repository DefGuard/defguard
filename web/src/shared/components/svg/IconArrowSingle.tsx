import type { SVGProps } from 'react';
const SvgIconArrowSingle = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={18}
    height={18}
    fill="none"
    viewBox="0 0 18 18"
    {...props}
  >
    <mask
      id="icon-arrow-single_svg__a"
      width={18}
      height={18}
      x={0}
      y={0}
      maskUnits="userSpaceOnUse"
      style={{
        maskType: 'luminance',
      }}
    >
      <path fill="#fff" d="M18 0H0v18h18V0Z" />
    </mask>
    <g fill="#899CA8" mask="url(#icon-arrow-single_svg__a)">
      <path d="M11.075 7.406 8.76 9.72a.818.818 0 1 0 1.157 1.157l2.314-2.314a.818.818 0 0 0-1.157-1.157Z" />
      <path d="M9.637 9.719 7.323 7.405a.818.818 0 0 0-1.157 1.157l2.314 2.314a.818.818 0 1 0 1.157-1.157Z" />
    </g>
  </svg>
);
export default SvgIconArrowSingle;
