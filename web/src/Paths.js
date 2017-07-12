import React, { Component } from 'react';
import axios from 'axios';

function plus_s(word, num) {
  return num + ' ' + word + (1 === num ? '' : 's');
}

class Paths extends Component {
  constructor(props) {
    super(props);
    this.state = {
      paths: [],
    }
  }
  componentDidMount() {
    let that = this;
    axios.get('/ds/paths/' + this.props.id).then(resp => {
      let paths = {};
      for (let full_paths of resp.data.paths) {
        const last_component = full_paths.pop();
        if (!paths[last_component]) {
          paths[last_component] = [];
        }
        paths[last_component].push(full_paths);
      }
      that.setState({paths});
    })
  }

  render() {
    let top_names = Object.keys(this.state.paths);
    top_names.sort();
    let lines = [];
    for (let top_name of top_names) {
      lines.push(<li>{top_name} ({plus_s('use', this.state.paths[top_name].length)})</li>);
    }
    return (
      <ul>{lines}</ul>
    );
  }
}

export default Paths;
