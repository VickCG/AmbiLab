use chrono::{Duration, NaiveDate};
use indicatif::{ProgressBar, ProgressStyle};
use rand::prelude::*;
use rayon::prelude::*;
use serde::Serialize;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::Path;

const OUTPUT_DIR: &str = "data-test";

const NUM_CUSTOMERS: usize = 50_000;
const NUM_PRODUCTS: usize = 5_000;
const NUM_ORDERS: usize = 200_000;
const NUM_ORDER_ITEMS: usize = 500_000;

#[derive(Serialize)]
struct Customer {
    customer_id: String,
    first_name: String,
    last_name: String,
    email: String,
    phone: String,
    region: String,
    country: String,
    city: String,
    segment: String,
    created_at: String,
    lifetime_value: f64,
}

#[derive(Serialize)]
struct Product {
    product_id: String,
    name: String,
    category: String,
    subcategory: String,
    brand: String,
    unit_price: f64,
    unit_cost: f64,
    margin_pct: f64,
}

#[derive(Serialize)]
struct Order {
    order_id: String,
    customer_id: String,
    order_date: String,
    ship_date: String,
    ship_mode: String,
    status: String,
    priority: String,
    channel: String,
    total_amount: f64,
    discount_amount: f64,
}

#[derive(Serialize)]
struct OrderItem {
    order_item_id: i64,
    order_id: String,
    product_id: String,
    quantity: i32,
    unit_price: f64,
    discount_pct: f64,
    tax_amount: f64,
    total_amount: f64,
    profit: f64,
}

#[derive(Serialize)]
struct AnalyticsData {
    customers: Vec<Customer>,
    products: Vec<Product>,
    orders: Vec<Order>,
    order_items: Vec<OrderItem>,
    metadata: Metadata,
}

#[derive(Serialize)]
struct Metadata {
    generated_at: String,
    total_records: usize,
    version: String,
}

fn main() {
    println!("🚀 AmbiLab Data Generator");
    println!("========================\n");

    let start = std::time::Instant::now();

    println!("Generating {} customers...", NUM_CUSTOMERS);
    let customers = generate_customers();
    println!("✓ Customers generated\n");

    println!("Generating {} products...", NUM_PRODUCTS);
    let products = generate_products();
    println!("✓ Products generated\n");

    println!("Generating {} orders...", NUM_ORDERS);
    let orders = generate_orders(&customers);
    println!("✓ Orders generated\n");

    println!("Generating {} order items...", NUM_ORDER_ITEMS);
    let order_items = generate_order_items(&orders, &products);
    println!("✓ Order items generated\n");

    let total_records = customers.len() + products.len() + orders.len() + order_items.len();

    let data = AnalyticsData {
        customers,
        products,
        orders,
        order_items,
        metadata: Metadata {
            generated_at: chrono::Utc::now().to_rfc3339(),
            total_records,
            version: "1.0.0".to_string(),
        },
    };

    let output_dir = Path::new(OUTPUT_DIR);
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).expect("Failed to create output directory");
    }

    let output_path = output_dir.join("analytics_data.json");
    println!("Writing to {}...", output_path.display());

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap());
    pb.set_message("Serializing data...");

    let file = File::create(&output_path).expect("Failed to create file");
    let writer = BufWriter::with_capacity(64 * 1024 * 1024, file);
    serde_json::to_writer(writer, &data).expect("Failed to write JSON");

    pb.finish_with_message("✓ JSON file written");

    let elapsed = start.elapsed();
    println!("\n========================");
    println!("✅ Generation complete!");
    println!("   Total records: {}", total_records);
    println!("   Time elapsed: {:.2}s", elapsed.as_secs_f64());
    println!("   Output: {}", output_path.display());
}

fn generate_customers() -> Vec<Customer> {
    let regions = ["North America", "Europe", "Asia Pacific", "Latin America", "Middle East"];
    let countries = [
        ("North America", vec!["USA", "Canada", "Mexico"]),
        ("Europe", vec!["UK", "Germany", "France", "Spain", "Italy"]),
        ("Asia Pacific", vec!["Japan", "China", "Australia", "Singapore", "India"]),
        ("Latin America", vec!["Brazil", "Argentina", "Colombia", "Chile"]),
        ("Middle East", vec!["UAE", "Saudi Arabia", "Israel", "Qatar"]),
    ];
    let segments = ["Consumer", "Corporate", "Small Business", "Enterprise", "Government"];
    let first_names = [
        "James", "Mary", "John", "Patricia", "Robert", "Jennifer", "Michael", "Linda",
        "William", "Elizabeth", "David", "Barbara", "Richard", "Susan", "Joseph", "Jessica",
        "Thomas", "Sarah", "Charles", "Karen", "Wei", "Yuki", "Mohammed", "Fatima",
        "Carlos", "Maria", "Hans", "Sophie", "Pierre", "Emma", "Raj", "Priya",
    ];
    let last_names = [
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
        "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson",
        "Thomas", "Taylor", "Moore", "Jackson", "Martin", "Lee", "Chen", "Wang",
        "Kim", "Nguyen", "Patel", "Singh", "Mueller", "Schmidt", "Tanaka", "Sato",
    ];
    let cities = [
        "New York", "Los Angeles", "Chicago", "Houston", "Phoenix", "London", "Paris",
        "Berlin", "Tokyo", "Sydney", "Singapore", "Dubai", "Sao Paulo", "Toronto",
        "Mumbai", "Shanghai", "Hong Kong", "Seoul", "Madrid", "Rome", "Amsterdam",
    ];

    let pb = ProgressBar::new(NUM_CUSTOMERS as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos}/{len} [{eta}]")
            .unwrap(),
    );

    let customers: Vec<Customer> = (0..NUM_CUSTOMERS)
        .into_par_iter()
        .map(|i| {
            let mut rng = rand::thread_rng();
            let region = regions.choose(&mut rng).unwrap();
            let country_list = countries.iter().find(|(r, _)| r == region).unwrap();
            let country = country_list.1.choose(&mut rng).unwrap();
            let first_name = first_names.choose(&mut rng).unwrap();
            let last_name = last_names.choose(&mut rng).unwrap();

            let base_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
            let days_offset = rng.gen_range(0..1500);
            let created = base_date + Duration::days(days_offset);

            Customer {
                customer_id: format!("CUST-{:06}", i + 1),
                first_name: first_name.to_string(),
                last_name: last_name.to_string(),
                email: format!("{}.{}{}@example.com", first_name.to_lowercase(), last_name.to_lowercase(), i),
                phone: format!("+1-{:03}-{:03}-{:04}", rng.gen_range(200..999), rng.gen_range(100..999), rng.gen_range(1000..9999)),
                region: region.to_string(),
                country: country.to_string(),
                city: cities.choose(&mut rng).unwrap().to_string(),
                segment: segments.choose(&mut rng).unwrap().to_string(),
                created_at: created.format("%Y-%m-%d").to_string(),
                lifetime_value: (rng.gen_range(100.0_f64..50000.0) * 100.0).round() / 100.0,
            }
        })
        .collect();

    pb.finish_and_clear();
    customers
}

fn generate_products() -> Vec<Product> {
    let categories = [
        ("Electronics", vec!["Smartphones", "Laptops", "Tablets", "Accessories", "Audio"]),
        ("Clothing", vec!["Men's Wear", "Women's Wear", "Kids", "Sportswear", "Footwear"]),
        ("Home & Garden", vec!["Furniture", "Kitchen", "Decor", "Garden", "Lighting"]),
        ("Office Supplies", vec!["Paper", "Writing", "Storage", "Technology", "Furniture"]),
        ("Sports", vec!["Fitness", "Outdoor", "Team Sports", "Water Sports", "Winter Sports"]),
    ];
    let brands = [
        "TechPro", "StyleMax", "HomeLife", "OfficePlus", "SportElite",
        "ValueBrand", "PremiumChoice", "EcoGreen", "UrbanStyle", "ClassicLine",
    ];
    let adjectives = ["Premium", "Professional", "Essential", "Advanced", "Basic", "Ultra", "Pro", "Elite"];
    let nouns = ["Edition", "Series", "Collection", "Line", "Model", "Version"];

    let pb = ProgressBar::new(NUM_PRODUCTS as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos}/{len} [{eta}]")
            .unwrap(),
    );

    let products: Vec<Product> = (0..NUM_PRODUCTS)
        .into_par_iter()
        .map(|i| {
            let mut rng = rand::thread_rng();
            let (category, subcats) = categories.choose(&mut rng).unwrap();
            let subcategory = subcats.choose(&mut rng).unwrap();
            let brand = brands.choose(&mut rng).unwrap();
            let adj = adjectives.choose(&mut rng).unwrap();
            let noun = nouns.choose(&mut rng).unwrap();

            let unit_cost = (rng.gen_range(5.0_f64..500.0) * 100.0).round() / 100.0;
            let margin: f64 = rng.gen_range(0.15..0.60);
            let unit_price = (unit_cost / (1.0 - margin) * 100.0).round() / 100.0;

            Product {
                product_id: format!("PROD-{:05}", i + 1),
                name: format!("{} {} {} {}", brand, adj, subcategory, noun),
                category: category.to_string(),
                subcategory: subcategory.to_string(),
                brand: brand.to_string(),
                unit_price,
                unit_cost,
                margin_pct: (margin * 100.0 * 100.0).round() / 100.0,
            }
        })
        .collect();

    pb.finish_and_clear();
    products
}

fn generate_orders(customers: &[Customer]) -> Vec<Order> {
    let ship_modes = ["Standard", "Express", "Same Day", "Economy", "Priority"];
    let statuses = ["Delivered", "Shipped", "Processing", "Pending", "Cancelled", "Returned"];
    let priorities = ["Low", "Medium", "High", "Critical"];
    let channels = ["Online", "Mobile App", "In-Store", "Phone", "Partner"];

    let pb = ProgressBar::new(NUM_ORDERS as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos}/{len} [{eta}]")
            .unwrap(),
    );

    let orders: Vec<Order> = (0..NUM_ORDERS)
        .into_par_iter()
        .map(|i| {
            let mut rng = rand::thread_rng();
            let customer = customers.choose(&mut rng).unwrap();

            let base_date = NaiveDate::from_ymd_opt(2022, 1, 1).unwrap();
            let days_offset = rng.gen_range(0..730);
            let order_date = base_date + Duration::days(days_offset);
            let ship_days = rng.gen_range(1..14);
            let ship_date = order_date + Duration::days(ship_days);

            let total = (rng.gen_range(25.0_f64..2500.0) * 100.0).round() / 100.0;
            let discount = if rng.gen_bool(0.3) {
                (total * rng.gen_range(0.05..0.25) * 100.0).round() / 100.0
            } else {
                0.0
            };

            Order {
                order_id: format!("ORD-{:07}", i + 1),
                customer_id: customer.customer_id.clone(),
                order_date: order_date.format("%Y-%m-%d").to_string(),
                ship_date: ship_date.format("%Y-%m-%d").to_string(),
                ship_mode: ship_modes.choose(&mut rng).unwrap().to_string(),
                status: statuses.choose(&mut rng).unwrap().to_string(),
                priority: priorities.choose(&mut rng).unwrap().to_string(),
                channel: channels.choose(&mut rng).unwrap().to_string(),
                total_amount: total,
                discount_amount: discount,
            }
        })
        .collect();

    pb.finish_and_clear();
    orders
}

fn generate_order_items(orders: &[Order], products: &[Product]) -> Vec<OrderItem> {
    let pb = ProgressBar::new(NUM_ORDER_ITEMS as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{bar:40.cyan/blue} {pos}/{len} [{eta}]")
            .unwrap(),
    );

    let order_items: Vec<OrderItem> = (0..NUM_ORDER_ITEMS)
        .into_par_iter()
        .map(|i| {
            let mut rng = rand::thread_rng();
            let order = orders.choose(&mut rng).unwrap();
            let product = products.choose(&mut rng).unwrap();

            let quantity = rng.gen_range(1..10);
            let discount_pct: f64 = if rng.gen_bool(0.25) {
                (rng.gen_range(0.05_f64..0.30) * 100.0).round() / 100.0
            } else {
                0.0
            };

            let subtotal = product.unit_price * quantity as f64;
            let discount_amt = subtotal * discount_pct;
            let tax_rate = 0.08;
            let tax_amount = ((subtotal - discount_amt) * tax_rate * 100.0).round() / 100.0;
            let total = ((subtotal - discount_amt + tax_amount) * 100.0).round() / 100.0;
            let profit = ((product.unit_price - product.unit_cost) * quantity as f64 - discount_amt).round() / 100.0 * 100.0;

            OrderItem {
                order_item_id: (i + 1) as i64,
                order_id: order.order_id.clone(),
                product_id: product.product_id.clone(),
                quantity,
                unit_price: product.unit_price,
                discount_pct,
                tax_amount,
                total_amount: total,
                profit: (profit * 100.0).round() / 100.0,
            }
        })
        .collect();

    pb.finish_and_clear();
    order_items
}
