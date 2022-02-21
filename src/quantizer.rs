use std::{cell::RefCell, rc::Rc};

use image::{Pixel, Rgb, RgbImage};

const MAX_LEVEL: usize = 5;

pub(crate) fn quantize(image: &RgbImage, num_colors: usize) -> RgbImage {
    let mut quantizer = OctTreeQuantizer::new(num_colors);
    quantizer.quantize(image)
}

struct OctTreeQuantizer {
    root: Rc<RefCell<OctTreeNode>>,
    reduce_colors: usize,
    maximum_colors: usize,
    colors: usize,
    color_list: Vec<Vec<Option<Rc<RefCell<OctTreeNode>>>>>,
}

impl OctTreeQuantizer {
    fn new(num_colors: usize) -> Self {
        let mut new_quantizer = Self {
            root: Rc::new(RefCell::new(OctTreeNode::new())),
            reduce_colors: usize::max(512, num_colors * 2),
            maximum_colors: num_colors,
            color_list: vec![],
            colors: 0,
        };

        for _ in 0..=MAX_LEVEL {
            new_quantizer.color_list.push(Vec::new());
        }
        new_quantizer
    }

    pub fn quantize(&mut self, image: &RgbImage) -> RgbImage {
        for pixel in image.pixels() {
            self.insert_color(pixel, Rc::clone(&self.root));

            if self.colors > self.reduce_colors {
                self.reduce_tree(self.reduce_colors);
                //reduce sets to None and the code below actually removes nodes from the list
                for level in &mut self.color_list {
                    level.retain(|c| c.is_some());
                }
            }
        }
        let table = self.build_color_table();

        let mut imgbuf = RgbImage::new(image.width(), image.height());
        for (x, y, pixel) in image.enumerate_pixels() {
            if let Some(index) = self.get_index_for_color(pixel, &self.root) {
                let color = &table[index];
                if let Some(color) = color {
                    imgbuf.put_pixel(x, y, *color);
                }
            }
        }

        imgbuf
    }

    fn get_index_for_color<P>(&self, color: &P, node: &Rc<RefCell<OctTreeNode>>) -> Option<usize>
        where
            P: Pixel<Subpixel = u8> + 'static,
    {
        fn get_index_for_color<P>(
            quantizer: &OctTreeQuantizer,
            color: &P,
            level: usize,
            node: &Rc<RefCell<OctTreeNode>>,
        ) -> Option<usize>
            where
                P: Pixel<Subpixel = u8> + 'static,
        {
            if level > MAX_LEVEL {
                return None;
            }
            let node = Rc::clone(node);
            let index = get_bitmask(color, &level);

            let node_b = node.borrow();
            let child = &node_b.leaf[index];

            if let Some(child) = child {
                let child_b = child.borrow();
                if child_b.is_leaf {
                    return Some(child_b.index);
                } else {
                    return get_index_for_color(quantizer, color, level + 1, child);
                }
            } else {
                return Some(node_b.index);
            }
        }

        get_index_for_color(&self, color, 0, node)
    }

    fn build_color_table(&mut self) -> Vec<Option<Rgb<u8>>> {
        //nested function that is called recursively
        fn build_color_table(
            quantizer: &mut OctTreeQuantizer,
            node: &Rc<RefCell<OctTreeNode>>,
            table: &mut Vec<Option<Rgb<u8>>>,
            index: usize,
        ) -> usize {
            if quantizer.colors > quantizer.maximum_colors {
                quantizer.reduce_tree(quantizer.maximum_colors);
            }
            if node.borrow().is_leaf {
                {
                    let node = node.borrow();
                    let count = node.count;
                    table[index] = Some(Rgb::from([
                        (node.total_red / count as u32) as u8,
                        (node.total_green / count as u32) as u8,
                        (node.total_blue / count as u32) as u8,
                    ]));
                }
                node.borrow_mut().index = index;

                index + 1
            } else {
                let mut result = index;
                for i in 0..8 {
                    // cannot iterate leaf, because that widens the scope of the borrow (of node)
                    let mut node = node.borrow_mut();
                    if let Some(leaf) = &node.leaf[i] {
                        //could be immutable borrow
                        let new_index = build_color_table(quantizer, leaf, table, result);
                        node.index = index; //but also need mutable borrow here
                        result = new_index;
                    }
                }

                result
            }
        }

        let mut table: Vec<Option<Rgb<u8>>> = vec![None; self.colors];
        let node = Rc::clone(&self.root);
        build_color_table(self, &node, &mut table, 0);
        table
    }

    fn insert_color<P>(&mut self, rgb: &P, node: Rc<RefCell<OctTreeNode>>)
        where
            P: Pixel<Subpixel = u8> + 'static,
    {
        //nested function that is called recursively
        fn insert_color<P>(
            quantizer: &mut OctTreeQuantizer,
            color: &P,
            level: usize,
            node: Rc<RefCell<OctTreeNode>>,
        ) where
            P: Pixel<Subpixel = u8> + 'static,
        {
            if level > MAX_LEVEL {
                return;
            }

            let index = get_bitmask(color, &level);

            if node.borrow().leaf[index].is_none() {
                let mut child = OctTreeNode::new();
                child.parent = Some(Rc::clone(&node));
                child.p_index = quantizer.color_list[level].len();

                if level == MAX_LEVEL {
                    child.is_leaf = true;
                    child.count = 1;
                    child.total_red = color.channels()[0] as u32;
                    child.total_green = color.channels()[1] as u32;
                    child.total_blue = color.channels()[2] as u32;
                    child.level = level;
                    quantizer.colors += 1;
                }

                let child = Rc::new(RefCell::new(child));
                quantizer.color_list[level].push(Some(Rc::clone(&child)));
                let clone = Rc::clone(&child);

                {
                    let mut mutnode = node.borrow_mut();
                    mutnode.children += 1;
                    mutnode.is_leaf = false;
                    mutnode.leaf[index] = Some(child);
                }

                if level < MAX_LEVEL {
                    insert_color(quantizer, color, level + 1, clone);
                } else {
                    return;
                }
            } else {
                if node
                    .borrow()
                    .leaf
                    .get(index)
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .is_leaf
                {
                    let mut node = node.borrow_mut();
                    let mut child = node
                        .leaf
                        .get_mut(index)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .borrow_mut();
                    child.count += 1;
                    child.total_red += color.channels()[0] as u32;
                    child.total_green += color.channels()[1] as u32;
                    child.total_blue += color.channels()[2] as u32;
                    return;
                } else {
                    insert_color(
                        quantizer,
                        color,
                        level + 1,
                        Rc::clone(&(node.borrow().leaf[index]).as_ref().unwrap()),
                    );
                }
            }
        }

        insert_color(self, rgb, 0, node);
    }

    fn reduce_tree(&mut self, num_colors: usize) {
        // Nested function that is called recursively
        fn reduce_tree(quantizer: &mut OctTreeQuantizer, num_colors: usize, level: isize) {
            if level < 0 {
                return;
            } else {
                let mut removals = Vec::new();
                let list = &quantizer.color_list[level as usize];
                for node in list {
                    if let Some(node) = node {
                        if node.borrow().children > 0 {
                            for i in 0..8 {
                                let mut color: Option<(usize, u32, u32, u32, usize)> = None;

                                if let Some(child) = node.borrow().leaf.get(i) {
                                    if let Some(child) = child {
                                        let child = child.borrow();
                                        color = Some((
                                            child.count,
                                            child.total_red,
                                            child.total_green,
                                            child.total_blue,
                                            child.p_index,
                                        ));
                                    }
                                }

                                // need to mutate node, which conflicts with previous borrow to retrieve the child
                                if let Some(color) = color {
                                    let mut node = node.borrow_mut();
                                    node.count += color.0;
                                    node.total_red += color.1;
                                    node.total_green += color.2;
                                    node.total_blue += color.3;
                                    node.leaf[i] = None;
                                    node.children -= 1;
                                    quantizer.colors -= 1;

                                    removals.push(color.4); //save for further processing outside loop (and borrow of colorlist)
                                }
                            }
                            node.borrow_mut().is_leaf = true;
                            quantizer.colors += 1;
                            if quantizer.colors <= num_colors {
                                return;
                            }
                        }
                    }
                }
                let color_list = &mut quantizer.color_list[(level as usize + 1)];
                for index in removals {
                    color_list[index] = None; //set to None here, Option removed later
                }

                reduce_tree(quantizer, num_colors, level - 1);
            }
        }

        // call to nested function
        reduce_tree(self, num_colors, (MAX_LEVEL - 1) as isize);
    }
}

struct OctTreeNode {
    children: usize,
    level: usize,
    parent: Option<Rc<RefCell<OctTreeNode>>>,
    leaf: Vec<Option<Rc<RefCell<OctTreeNode>>>>,
    is_leaf: bool,
    count: usize,
    total_red: u32,
    total_green: u32,
    total_blue: u32,
    index: usize,
    p_index: usize,
}

impl OctTreeNode {
    fn new() -> Self {
        Self {
            children: 0,
            level: 0,
            parent: None,
            leaf: vec![None; 8],
            is_leaf: false,
            count: 0,
            total_red: 0,
            total_green: 0,
            total_blue: 0,
            index: 0,
            p_index: 0,
        }
    }
}

fn get_bitmask<P>(color: &P, level: &usize) -> usize
    where
        P: Pixel<Subpixel = u8> + 'static,
{
    let bit = 0x80 >> level;

    let mut index = 0;
    if (color.channels()[0] & bit) != 0 {
        index += 4;
    }
    if (color.channels()[1] & bit) != 0 {
        index += 2;
    }
    if (color.channels()[2] & bit) != 0 {
        index += 1;
    }
    index
}
