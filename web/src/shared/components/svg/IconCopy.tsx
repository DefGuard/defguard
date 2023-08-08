import type { SVGProps } from 'react';
const SvgIconCopy = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path fill="#899ca8" d="M0 0h22v22H0z" data-name="Rectangle 2627" opacity={0} />
      </clipPath>
    </defs>
    <g clipPath="url(#a)" transform="rotate(90 11 11)">
      <g fill="#899ca8" data-name="Group 4639" transform="rotate(-90 53.5 280.5)">
        <rect
          width={10}
          height={2}
          data-name="Rectangle 2621"
          rx={1}
          transform="rotate(90 41.5 276.5)"
        />
        <rect
          width={10}
          height={2}
          data-name="Rectangle 2628"
          rx={1}
          transform="rotate(90 45.5 280.5)"
        />
        <rect
          width={8}
          height={2}
          data-name="Rectangle 2633"
          rx={1}
          transform="rotate(90 49.5 280.5)"
        />
        <rect
          width={10}
          height={2}
          data-name="Rectangle 2629"
          rx={1}
          transform="rotate(180 163 118.5)"
        />
        <rect
          width={8}
          height={2}
          data-name="Rectangle 2634"
          rx={1}
          transform="rotate(180 165 116.5)"
        />
        <rect
          width={10}
          height={2}
          data-name="Rectangle 2630"
          rx={1}
          transform="rotate(180 163 122.5)"
        />
      </g>
    </g>
  </svg>
);
export default SvgIconCopy;
