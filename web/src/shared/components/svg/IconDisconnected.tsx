import type { SVGProps } from 'react';
const SvgIconDisconnected = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="icon-disconnected_svg__a">
        <path
          d="M0 0h16v16H0z"
          style={{
            fill: '#cb3f3f',
          }}
        />
      </clipPath>
    </defs>
    <g
      style={{
        clipPath: 'url(#icon-disconnected_svg__a)',
      }}
    >
      <circle
        cx={3}
        cy={3}
        r={3}
        style={{
          fill: '#899ca8',
        }}
        transform="translate(5 5)"
      />
    </g>
  </svg>
);
export default SvgIconDisconnected;
