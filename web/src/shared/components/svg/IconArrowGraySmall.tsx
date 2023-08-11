import type { SVGProps } from 'react';
const SvgIconArrowGraySmall = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="icon-arrow-gray-small_svg__a">
        <path fill="#899ca8" d="M0 0h22v22H0z" data-name="Rectangle 2812" opacity={0} />
      </clipPath>
    </defs>
    <g fill="#899ca8" clipPath="url(#icon-arrow-gray-small_svg__a)">
      <rect
        width={6}
        height={2}
        data-name="Rectangle 2810"
        rx={1}
        transform="rotate(-135 8.9 3.586)"
      />
      <rect
        width={6}
        height={2}
        data-name="Rectangle 2811"
        rx={1}
        transform="rotate(-45 21.314 -3.07)"
      />
    </g>
  </svg>
);
export default SvgIconArrowGraySmall;
