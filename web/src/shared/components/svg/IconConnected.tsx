import type { SVGProps } from 'react';
const SvgIconConnected = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={16}
    height={16}
    viewBox="0 0 16 16"
    {...props}
  >
    <defs>
      <clipPath id="a">
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
        clipPath: 'url(#a)',
      }}
    >
      <g
        style={{
          stroke: '#14bc6e',
          strokeWidth: 2,
          fill: 'none',
        }}
        transform="translate(4 4)"
      >
        <circle
          cx={4}
          cy={4}
          r={4}
          stroke="none"
          style={{
            stroke: 'none',
          }}
        />
        <circle
          cx={4}
          cy={4}
          r={3}
          style={{
            fill: 'none',
          }}
        />
      </g>
    </g>
  </svg>
);
export default SvgIconConnected;
