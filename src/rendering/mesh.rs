use crate::gl;
use gltf;
use std::path::Path;

enum IndexType {
    UnsignedByte,
    UnsignedShort,
    UnsignedInt,
}

pub struct Mesh {
    buffer: u32,
    vao: u32,
    num_triangles: u32,
    index_type: IndexType,
    textures: [u32; 4],
    model_matrix: [f32; 16],
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            buffer: 0,
            vao: 0,
            num_triangles: 0,
            index_type: IndexType::UnsignedShort,
            textures: [0, 0, 0, 0],
            model_matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn add_vertices(&mut self, vertices: &[u8], indices: &[u8]) {
        let vert_len_aligned = vertices.len() + vertices.len() % 4;
        let ind_len = match self.index_type {
            IndexType::UnsignedByte => 1,
            IndexType::UnsignedShort => 2,
            IndexType::UnsignedInt => 4,
        };
        self.num_triangles = (indices.len() / (ind_len * 3)) as u32;

        let ind_len_aligned = indices.len();
        let total_buffer_size = vertices.len() + indices.len();
        println!("Allocating a total buffer of size: {}", total_buffer_size);
        unsafe {
            gl::CreateBuffers(1, &mut self.buffer);
            gl::NamedBufferStorage(
                self.buffer,
                total_buffer_size as isize,
                std::ptr::null(),
                gl::DYNAMIC_STORAGE_BIT,
            );
            println!(
                "Indices: {} triangles, uploading {} bytes at offset: 0",
                self.num_triangles, ind_len_aligned
            );
            gl::NamedBufferSubData(
                self.buffer,
                0,
                indices.len() as isize,
                indices.as_ptr() as *const _,
            );
            println!(
                "Vertices: uploading {} bytes at offset: {}",
                vertices.len(),
                ind_len_aligned
            );
            gl::NamedBufferSubData(
                self.buffer,
                ind_len_aligned as isize,
                vertices.len() as isize,
                vertices.as_ptr() as *const _,
            );

            gl::CreateVertexArrays(1, &mut self.vao);
            gl::VertexArrayVertexBuffer(self.vao, 0, self.buffer, ind_len_aligned as isize, 24);
            gl::VertexArrayElementBuffer(self.vao, self.buffer);

            //TODO: Read these from GLTF spec?
            gl::EnableVertexArrayAttrib(self.vao, 0);
            gl::EnableVertexArrayAttrib(self.vao, 1);

            gl::VertexArrayAttribFormat(self.vao, 0, 3, gl::FLOAT, gl::FALSE, 0);
            gl::VertexArrayAttribFormat(self.vao, 1, 3, gl::FLOAT, gl::FALSE, 12);

            gl::VertexArrayAttribBinding(self.vao, 0, 0);
            gl::VertexArrayAttribBinding(self.vao, 1, 0);
        }
    }

    fn parse_node(&mut self, node: &gltf::Node, buffers: &Vec<gltf::buffer::Data>) {
        // node.json.
        if let Some(nodename) = node.name() {
            println!("Got node: {}", nodename);
        }
        if let Some(mesh) = node.mesh() {
            println!("Found mesh {:?} in node!", mesh.name());
            let mut index_arr: &[u8] = &[0u8];
            let mut pos_arr: &[u8] = &[0u8];
            let mut normal_arr: &[u8] = &[0u8];
            for primitive in mesh.primitives() {
                if let Some(indices) = primitive.indices() {
                    let ind_offset = indices.view().offset();
                    let ind_size = indices.view().length();
                    let acc_size = indices.size();
                    if acc_size == 1 {
                        self.index_type = IndexType::UnsignedByte;
                    } else if acc_size == 2 {
                        self.index_type = IndexType::UnsignedShort;
                    } else if acc_size == 4 {
                        self.index_type = IndexType::UnsignedInt;
                    } else {
                        panic!("Cannot parse this node");
                    }
                    println!(
                        "Want an index buffer of stride: {}, with offset: {}, total bytelen: {}",
                        acc_size, ind_offset, ind_size
                    );
                    let buf_index = indices.view().buffer().index();
                    let ind_buf = &buffers[buf_index];
                    index_arr = &ind_buf[ind_offset..ind_offset + ind_size];
                }
                let mut start_index;
                let mut end_index;

                //TODO: upload all primitives, but only use the ones we can...
                for attribute in primitive.attributes() {
                    //Striding needs to be acknowledged
                    let accessor = attribute.1;
                    let acc_view = accessor.view();
                    let acc_total_size = accessor.size() * accessor.count();
                    let buf_index = acc_view.buffer().index();
                    match attribute.0 {
                        gltf::json::mesh::Semantic::Positions => {
                            //TODO: This accessor for Box.glb should return a byte_offset of 288B!
                            start_index = acc_view.offset();
                            end_index = start_index + acc_total_size;
                            let pos_buf = &buffers[buf_index];
                            pos_arr = &pos_buf[start_index..end_index];
                        }
                        gltf::json::mesh::Semantic::Normals => {
                            start_index = acc_view.offset();
                            end_index = start_index + acc_total_size;
                            let pos_buf = &buffers[buf_index];
                            normal_arr = &pos_buf[start_index..end_index];
                        }
                        _ => {}
                    }
                }
            }
            let vert_vec = pos_arr
                .chunks(12)
                .zip(normal_arr.chunks(12))
                .flat_map(|(a, b)| a.into_iter().chain(b))
                .copied()
                .collect::<Vec<u8>>();

            self.add_vertices(&vert_vec[..], index_arr);
        }
    }

    fn parse_scene() {
        unimplemented!("Cannot yet parse the scene")
    }

    pub fn read_gltf<P>(&mut self, path: P)
    where
        P: AsRef<Path>,
    {
        let (document, buffers, _images) = gltf::import(path).unwrap();
        let mut used_nodes = vec![];
        for scene in document.scenes() {
            // parse_scene();
            for node in scene.nodes() {
                println!("Scene #{} uses node #{}", scene.index(), node.index());
                used_nodes.push(node.index());
                for child in node.children() {
                    used_nodes.push(child.index());
                }
            }
        }
        for node in document.nodes() {
            if used_nodes.contains(&node.index()) {
                self.parse_node(&node, &buffers);
            }
        }
        for buffer_desc in document.buffers() {
            println!(
                "Buffer id {} has bytelen: {}",
                buffer_desc.index(),
                buffer_desc.length()
            );
            println!("Buffer index: {}", buffers[0].len());
        }
    }

    pub fn draw(&self) {
        unsafe {
            gl::BindVertexArray(self.vao);
            match self.index_type {
                IndexType::UnsignedByte => {
                    gl::DrawElements(
                        gl::TRIANGLES,
                        self.num_triangles as i32,
                        gl::UNSIGNED_BYTE,
                        std::ptr::null(),
                    );
                }
                IndexType::UnsignedShort => {
                    gl::DrawElements(
                        gl::TRIANGLES,
                        (self.num_triangles * 2) as i32,
                        gl::UNSIGNED_SHORT,
                        std::ptr::null(),
                    );
                }
                IndexType::UnsignedInt => {
                    gl::DrawElements(
                        gl::TRIANGLES,
                        (self.num_triangles * 4) as i32,
                        gl::UNSIGNED_INT,
                        std::ptr::null(),
                    );
                }
            }
        }
    }
}
