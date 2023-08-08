import type { SVGProps } from 'react';
const SvgIconUserListElement = (props: SVGProps<SVGSVGElement>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    width={22}
    height={22}
    viewBox="0 0 22 22"
    {...props}
  >
    <defs>
      <clipPath id="a">
        <path
          d="M0 0h22v22H0z"
          style={{
            opacity: 0,
            fill: '#899ca8',
          }}
        />
      </clipPath>
      <style>{'.c{fill:#899ca8}'}</style>
    </defs>
    <g
      style={{
        clipPath: 'url(#a)',
      }}
      transform="translate(-313 -224.475)"
    >
      <rect
        width={10}
        height={2}
        className="c"
        rx={1}
        transform="translate(319 237.475)"
      />
      <rect
        width={5}
        height={2}
        className="c"
        rx={1}
        transform="rotate(45 -119.83 511.254)"
      />
      <rect
        width={5}
        height={2}
        className="c"
        rx={1}
        transform="rotate(-45 453.425 -271.805)"
      />
      <rect
        width={10}
        height={2}
        className="c"
        rx={1}
        transform="rotate(90 45.762 275.238)"
      />
    </g>
  </svg>
);
export default SvgIconUserListElement;
