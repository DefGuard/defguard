import type { SVGProps } from 'react';
const SvgIconCancel = (props: SVGProps<SVGSVGElement>) => (
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
      transform="rotate(90 11 11)"
    >
      <rect
        width={16}
        height={2}
        className="c"
        rx={1}
        transform="rotate(45 -2.571 9.621)"
      />
      <rect
        width={16}
        height={2}
        className="c"
        rx={1}
        transform="rotate(135 7.429 6.621)"
      />
    </g>
  </svg>
);
export default SvgIconCancel;
