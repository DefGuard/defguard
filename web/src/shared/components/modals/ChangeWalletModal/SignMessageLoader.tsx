import LoaderSpinner from '../../layout/LoaderSpinner/LoaderSpinner';

const SignMessageLoader: React.FC = () => {
  return (
    <div className="signing-loader-container">
      <LoaderSpinner size={116} />
      <div className="signing-loader">
        <h3 className="signing-loader-header">
          Please sign verification message
        </h3>
        <p className="signing-loader-text">
          Please check your mobile or browser extension, and
          <br />
          sign verification message.
        </p>
      </div>
    </div>
  );
};

export default SignMessageLoader;
