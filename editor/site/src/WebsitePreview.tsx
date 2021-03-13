import {h} from 'preact';

export default function WebsitePreview({ url }) {
  return <iframe
    class="preview"
    src={url}
    frameborder="0"
    sandbox="allow-scripts allow-forms"
    />
}
