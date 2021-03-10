import {h} from 'preact';
import {useRoute} from 'wouter';
import {Link} from 'wouter';

function Close(props) {
  return <Link href="/">
    <a href="#">Close [X]</a>
  </Link>;
}

export default function ProjectView() {
  const [match, params] = useRoute("/project/:id");
  return <div>
    <Close />
    <p>Project, {params.id}!</p>
  </div>
}
