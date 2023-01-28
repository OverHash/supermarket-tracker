# New World

The New World API is slightly better designed than Countdown's API.

## Common Api

```
https://www.newworld.co.nz/CommonApi/Cart/Index
```

- Retrieves details about your current cart (fees, store selected, subtotal, savings)

```
https://www.newworld.co.nz/CommonApi/Navigation/MegaMenu
```

- Retrieves all the categories available
- Has optional queries
  - `storeId` to represent the store you are querying items about
  - `v` to indicate verbose(?). This doesn't have any value.

```
https://www.newworld.co.nz/CommonApi/Store/GetStoreList
```

- Retrieves all the stores available

```
https://www.newworld.co.nz/CommonApi/Store/GetStoreServiceFees
```

- Retrieves the fees for collect and delivery.

## Brands Api

```
https://www.newworld.co.nz/BrandsApi/Algoliasearch/GetGroceriesItems
```

- has an optional query `searchTerm`
