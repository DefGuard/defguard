import type { SVGProps } from 'react';
const SvgIconTagDismiss = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="icon-tag-dismiss_svg__a">
        <path fill="#cbd3d8" d="M0 0h16v16H0z" data-name="Rectangle 2114" />
      </clipPath>
    </defs>
    <g fill="#cbd3d8" clipPath="url(#icon-tag-dismiss_svg__a)">
      <rect
        width={10}
        height={2}
        data-name="Rectangle 2113"
        rx={1}
        transform="rotate(135 5.05 5.122)"
      />
      <rect
        width={10}
        height={2}
        data-name="Rectangle 2156"
        rx={1}
        transform="rotate(-135 7.95 3.879)"
      />
    </g>
  </svg>
);
export default SvgIconTagDismiss;
