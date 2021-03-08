import {h, Fragment} from 'preact';
import {Link} from 'wouter';

function CloseCreate(props) {
  return <Link href="/">
    <a href="#">Close [X]</a>
  </Link>;
}

function CreateProject(props) {
  return <div>
    <CloseCreate />
    <p>Create new project</p>
  </div>;
}

export {CreateProject}
